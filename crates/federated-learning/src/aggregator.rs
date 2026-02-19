use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model weights representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWeights {
    pub layers: Vec<LayerWeights>,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerWeights {
    pub name: String,
    pub weights: Vec<f64>,
    pub shape: Vec<usize>,
}

impl ModelWeights {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            version: 0,
        }
    }

    /// Get total number of parameters
    pub fn num_parameters(&self) -> usize {
        self.layers.iter().map(|l| l.weights.len()).sum()
    }
}

impl Default for ModelWeights {
    fn default() -> Self {
        Self::new()
    }
}

/// Federated Learning Aggregator using FedAvg algorithm
pub struct FederatedAggregator {
    global_model: ModelWeights,
    client_contributions: HashMap<String, (ModelWeights, usize)>,
    round: u64,
}

impl FederatedAggregator {
    pub fn new(initial_model: ModelWeights) -> Self {
        Self {
            global_model: initial_model,
            client_contributions: HashMap::new(),
            round: 0,
        }
    }

    /// Add a client's model update
    pub fn add_client_update(
        &mut self,
        client_id: String,
        model: ModelWeights,
        num_samples: usize,
    ) -> Result<()> {
        if model.layers.len() != self.global_model.layers.len() {
            anyhow::bail!(
                "Client model has {} layers but global model has {} layers",
                model.layers.len(),
                self.global_model.layers.len()
            );
        }
        for (layer_idx, (client_layer, global_layer)) in
            model.layers.iter().zip(self.global_model.layers.iter()).enumerate()
        {
            if client_layer.weights.len() != global_layer.weights.len() {
                anyhow::bail!(
                    "Client model layer {} has {} weights but global model has {}",
                    layer_idx,
                    client_layer.weights.len(),
                    global_layer.weights.len()
                );
            }
        }
        self.client_contributions
            .insert(client_id, (model, num_samples));
        Ok(())
    }

    /// Aggregate all client updates using Federated Averaging (FedAvg)
    pub fn aggregate(&mut self) -> Result<ModelWeights> {
        if self.client_contributions.is_empty() {
            anyhow::bail!("No client contributions to aggregate");
        }

        // Calculate total samples across all clients
        let total_samples: usize = self
            .client_contributions
            .values()
            .map(|(_, samples)| samples)
            .sum();

        // Initialize aggregated weights
        let num_layers = self.global_model.layers.len();
        let mut aggregated_layers = Vec::with_capacity(num_layers);

        // Aggregate each layer
        for layer_idx in 0..num_layers {
            let layer_name = &self.global_model.layers[layer_idx].name;
            let layer_shape = &self.global_model.layers[layer_idx].shape;
            let num_weights = self.global_model.layers[layer_idx].weights.len();

            let mut aggregated_weights = vec![0.0; num_weights];

            // Weighted average of all client weights
            for (client_model, num_samples) in self.client_contributions.values() {
                let weight_factor = *num_samples as f64 / total_samples as f64;
                let client_layer = &client_model.layers[layer_idx];

                for (i, &weight) in client_layer.weights.iter().enumerate() {
                    aggregated_weights[i] += weight * weight_factor;
                }
            }

            aggregated_layers.push(LayerWeights {
                name: layer_name.clone(),
                weights: aggregated_weights,
                shape: layer_shape.clone(),
            });
        }

        self.round += 1;
        self.global_model = ModelWeights {
            layers: aggregated_layers,
            version: self.round,
        };

        // Clear contributions for next round
        self.client_contributions.clear();

        Ok(self.global_model.clone())
    }

    /// Get the current global model
    pub fn global_model(&self) -> &ModelWeights {
        &self.global_model
    }

    /// Get current round number
    pub fn current_round(&self) -> u64 {
        self.round
    }

    /// Get number of pending client contributions
    pub fn pending_contributions(&self) -> usize {
        self.client_contributions.len()
    }
}

impl Default for FederatedAggregator {
    fn default() -> Self {
        Self::new(ModelWeights::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_model(scale: f64) -> ModelWeights {
        ModelWeights {
            layers: vec![
                LayerWeights {
                    name: "layer1".to_string(),
                    weights: vec![1.0 * scale, 2.0 * scale, 3.0 * scale],
                    shape: vec![3],
                },
                LayerWeights {
                    name: "layer2".to_string(),
                    weights: vec![4.0 * scale, 5.0 * scale],
                    shape: vec![2],
                },
            ],
            version: 0,
        }
    }

    #[test]
    fn test_federated_aggregation() {
        let initial_model = create_test_model(1.0);
        let mut aggregator = FederatedAggregator::new(initial_model);

        // Add two client updates with equal samples
        aggregator
            .add_client_update("client1".to_string(), create_test_model(2.0), 100)
            .unwrap();
        aggregator
            .add_client_update("client2".to_string(), create_test_model(4.0), 100)
            .unwrap();

        // Aggregate
        let aggregated = aggregator.aggregate().unwrap();

        // With equal weights, should be average: (2.0 + 4.0) / 2 = 3.0
        assert_eq!(aggregated.layers[0].weights[0], 3.0);
        assert_eq!(aggregated.layers[0].weights[1], 6.0);
        assert_eq!(aggregated.layers[1].weights[0], 12.0);

        assert_eq!(aggregator.current_round(), 1);
        assert_eq!(aggregator.pending_contributions(), 0);
    }

    #[test]
    fn test_weighted_aggregation() {
        let initial_model = create_test_model(1.0);
        let mut aggregator = FederatedAggregator::new(initial_model);

        // Add two client updates with different sample counts
        aggregator
            .add_client_update("client1".to_string(), create_test_model(2.0), 100)
            .unwrap();
        aggregator
            .add_client_update("client2".to_string(), create_test_model(4.0), 200)
            .unwrap();

        // Aggregate
        let aggregated = aggregator.aggregate().unwrap();

        // Weighted average: (2.0 * 100 + 4.0 * 200) / 300 = (200 + 800) / 300 = 3.333...
        assert!((aggregated.layers[0].weights[0] - 3.333333).abs() < 0.001);
    }

    #[test]
    fn test_add_client_update_rejects_mismatched_layer_count() {
        let initial_model = create_test_model(1.0);
        let mut aggregator = FederatedAggregator::new(initial_model);

        // Build a model with a different number of layers than the global model
        let wrong_model = ModelWeights {
            layers: vec![LayerWeights {
                name: "only_layer".to_string(),
                weights: vec![1.0, 2.0, 3.0],
                shape: vec![3],
            }],
            version: 0,
        };

        let result = aggregator.add_client_update("client1".to_string(), wrong_model, 100);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("layers"), "error should mention layers: {}", msg);
    }

    #[test]
    fn test_add_client_update_rejects_mismatched_weight_count() {
        let initial_model = create_test_model(1.0);
        let mut aggregator = FederatedAggregator::new(initial_model);

        // Same layer count but wrong weight count in layer 0
        let wrong_model = ModelWeights {
            layers: vec![
                LayerWeights {
                    name: "layer1".to_string(),
                    weights: vec![1.0, 2.0], // should be 3 weights
                    shape: vec![2],
                },
                LayerWeights {
                    name: "layer2".to_string(),
                    weights: vec![4.0, 5.0],
                    shape: vec![2],
                },
            ],
            version: 0,
        };

        let result = aggregator.add_client_update("client1".to_string(), wrong_model, 100);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("weights"), "error should mention weights: {}", msg);
    }
}

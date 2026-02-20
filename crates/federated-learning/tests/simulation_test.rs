//! Simulation tests for the Federated Learning subsystem.
//!
//! Each test builds a realistic training scenario and asserts on the concrete
//! return values produced by the FedAvg aggregation algorithm, demonstrating
//! what the technology actually returns at runtime.

use federated_learning::{FederatedAggregator, LayerWeights, ModelWeights};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Build a simple two-layer model with controllable weight values.
fn make_model(layer1_scale: f64, layer2_scale: f64) -> ModelWeights {
    ModelWeights {
        layers: vec![
            LayerWeights {
                name: "dense_1".to_string(),
                weights: vec![
                    1.0 * layer1_scale,
                    2.0 * layer1_scale,
                    3.0 * layer1_scale,
                    4.0 * layer1_scale,
                ],
                shape: vec![2, 2],
            },
            LayerWeights {
                name: "output".to_string(),
                weights: vec![1.0 * layer2_scale, 2.0 * layer2_scale],
                shape: vec![2],
            },
        ],
        version: 0,
    }
}

// ---------------------------------------------------------------------------
// Basic aggregation returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_fedavg_equal_weight_returns() {
    let initial = make_model(1.0, 1.0);
    let mut aggregator = FederatedAggregator::new(initial);

    // Two clients with equal sample counts → simple arithmetic mean.
    aggregator
        .add_client_update("client-a".to_string(), make_model(2.0, 2.0), 100)
        .unwrap();
    aggregator
        .add_client_update("client-b".to_string(), make_model(4.0, 4.0), 100)
        .unwrap();

    assert_eq!(aggregator.pending_contributions(), 2);

    let global = aggregator.aggregate().unwrap();

    // Each weight in layer 1 should equal (2*scale + 4*scale) / 2 = 3*scale.
    let expected_l1 = [3.0, 6.0, 9.0, 12.0];
    for (i, &w) in global.layers[0].weights.iter().enumerate() {
        assert!(
            (w - expected_l1[i]).abs() < 1e-9,
            "layer1 weight[{i}]: expected {}, got {w}",
            expected_l1[i]
        );
    }

    // Layer 2 expected = (2.0 + 4.0) / 2 = 3.0, (4.0 + 8.0) / 2 = 6.0
    assert!((global.layers[1].weights[0] - 3.0).abs() < 1e-9);
    assert!((global.layers[1].weights[1] - 6.0).abs() < 1e-9);

    // Round counter advances.
    assert_eq!(global.version, 1);
    assert_eq!(aggregator.current_round(), 1);
    // Contributions cleared after aggregate.
    assert_eq!(aggregator.pending_contributions(), 0);
}

#[test]
fn simulate_fedavg_weighted_by_samples_returns() {
    let initial = make_model(1.0, 1.0);
    let mut aggregator = FederatedAggregator::new(initial);

    // client-a has 100 samples; client-b has 300 samples (3× more influence).
    aggregator
        .add_client_update("client-a".to_string(), make_model(1.0, 1.0), 100)
        .unwrap();
    aggregator
        .add_client_update("client-b".to_string(), make_model(5.0, 5.0), 300)
        .unwrap();

    let global = aggregator.aggregate().unwrap();

    // Weighted average = (1.0*100 + 5.0*300) / 400 = 1600/400 = 4.0 for scale=1.
    let expected_w0 = (1.0 * 100.0 + 5.0 * 300.0) / 400.0;
    assert!(
        (global.layers[0].weights[0] - expected_w0).abs() < 1e-9,
        "weighted mean mismatch: expected {expected_w0}, got {}",
        global.layers[0].weights[0]
    );
}

// ---------------------------------------------------------------------------
// Multi-round training returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_multi_round_returns() {
    let initial = make_model(0.0, 0.0);
    let mut aggregator = FederatedAggregator::new(initial);

    for round in 1u64..=3 {
        let scale = round as f64;
        aggregator
            .add_client_update(
                format!("client-{round}"),
                make_model(scale, scale),
                100,
            )
            .unwrap();

        let global = aggregator.aggregate().unwrap();
        assert_eq!(global.version, round, "version must match round number");
        assert_eq!(aggregator.current_round(), round);
    }

    // After 3 rounds the global model version is 3.
    assert_eq!(aggregator.global_model().version, 3);
}

// ---------------------------------------------------------------------------
// Error path returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_aggregate_with_no_contributions_returns_error() {
    let initial = make_model(1.0, 1.0);
    let mut aggregator = FederatedAggregator::new(initial);

    let result = aggregator.aggregate();
    assert!(result.is_err(), "aggregating with no contributions must fail");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No client contributions"),
        "error message should mention missing contributions"
    );
}

#[test]
fn simulate_mismatched_layer_count_returns_error() {
    let initial = make_model(1.0, 1.0);
    let mut aggregator = FederatedAggregator::new(initial);

    // Model with only one layer instead of two.
    let bad_model = ModelWeights {
        layers: vec![LayerWeights {
            name: "dense_1".to_string(),
            weights: vec![1.0, 2.0, 3.0, 4.0],
            shape: vec![2, 2],
        }],
        version: 0,
    };

    let result = aggregator.add_client_update("bad-client".to_string(), bad_model, 50);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("layers"));
}

#[test]
fn simulate_mismatched_weight_count_returns_error() {
    let initial = make_model(1.0, 1.0);
    let mut aggregator = FederatedAggregator::new(initial);

    // Correct layer count but wrong weight count in layer 0.
    let bad_model = ModelWeights {
        layers: vec![
            LayerWeights {
                name: "dense_1".to_string(),
                weights: vec![1.0, 2.0], // should have 4 weights
                shape: vec![1, 2],
            },
            LayerWeights {
                name: "output".to_string(),
                weights: vec![1.0, 2.0],
                shape: vec![2],
            },
        ],
        version: 0,
    };

    let result = aggregator.add_client_update("bad-client".to_string(), bad_model, 50);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("weights"));
}

// ---------------------------------------------------------------------------
// num_parameters helper returns
// ---------------------------------------------------------------------------

#[test]
fn simulate_num_parameters_returns() {
    let model = make_model(1.0, 1.0);
    // layer1: 4 weights + layer2: 2 weights = 6 total.
    assert_eq!(model.num_parameters(), 6);
}

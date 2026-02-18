//! Trust scoring for model outputs

use serde::{Deserialize, Serialize};

use super::adapters::ModelOutput;

/// Trust scores for model outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScores {
    /// Confidence score from model (0.0 - 1.0)
    pub confidence_score: f64,
    /// Safety score (0.0 - 1.0)
    pub safety_score: f64,
    /// Consistency score vs peer outputs (0.0 - 1.0)
    pub consistency_score: f64,
}

impl TrustScores {
    /// Create new trust scores
    pub fn new(confidence_score: f64, safety_score: f64, consistency_score: f64) -> Self {
        Self {
            confidence_score: confidence_score.clamp(0.0, 1.0),
            safety_score: safety_score.clamp(0.0, 1.0),
            consistency_score: consistency_score.clamp(0.0, 1.0),
        }
    }

    /// Compute overall trust score (weighted average)
    pub fn overall_score(&self) -> f64 {
        // Weights: confidence 40%, safety 30%, consistency 30%
        (self.confidence_score * 0.4) + (self.safety_score * 0.3) + (self.consistency_score * 0.3)
    }
}

impl Default for TrustScores {
    fn default() -> Self {
        Self {
            confidence_score: 0.5,
            safety_score: 1.0, // Assume safe by default
            consistency_score: 0.5,
        }
    }
}

/// Consistency score computation
pub struct ConsistencyScore;

impl ConsistencyScore {
    /// Compute semantic similarity between two texts
    ///
    /// **STUB IMPLEMENTATION**: This is a placeholder for demonstration purposes.
    ///
    /// Production implementations should use:
    /// - Sentence embeddings (e.g., Sentence-BERT, USE)
    /// - Cosine similarity on embedding vectors
    /// - Pre-trained language models
    /// - Levenshtein distance for character-level similarity
    ///
    /// Current approach has known limitations:
    /// - Only counts character overlap, not semantic meaning
    /// - Doesn't consider word boundaries or order
    /// - Inefficient HashSet allocation per call
    pub fn compute_similarity(text1: &str, text2: &str) -> f64 {
        // Simple stub: normalized character overlap
        let len1 = text1.len();
        let len2 = text2.len();

        if len1 == 0 && len2 == 0 {
            return 1.0;
        }

        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        // Convert text2 to HashSet for O(1) lookups
        // NOTE: This is inefficient and should be replaced with proper similarity algorithm
        use std::collections::HashSet;
        let text2_chars: HashSet<char> = text2.chars().collect();

        // Count common characters
        let common = text1.chars().filter(|c| text2_chars.contains(c)).count();

        let max_len = len1.max(len2) as f64;
        (common as f64 / max_len).clamp(0.0, 1.0)
    }

    /// Compute consistency score for an output against a set of peer outputs
    pub fn compute_consistency(output: &ModelOutput, peers: &[ModelOutput]) -> f64 {
        if peers.is_empty() {
            return 0.5; // Neutral score if no peers
        }

        let similarities: Vec<f64> = peers
            .iter()
            .map(|peer| Self::compute_similarity(&output.text, &peer.text))
            .collect();

        // Average similarity
        let sum: f64 = similarities.iter().sum();
        (sum / similarities.len() as f64).clamp(0.0, 1.0)
    }
}

/// Safety checker for model outputs
pub struct SafetyChecker;

impl SafetyChecker {
    /// Check if output is safe
    ///
    /// **STUB IMPLEMENTATION**: This is a placeholder for demonstration purposes.
    ///
    /// Production implementations should use:
    /// - Professional content moderation APIs (e.g., OpenAI Moderation, Perspective API)
    /// - Multi-category safety classification (hate speech, violence, etc.)
    /// - Text normalization to prevent bypassing (l33tspeak, unicode tricks)
    /// - Context-aware analysis (not just keyword matching)
    /// - Regular updates to safety patterns
    ///
    /// Current approach has critical limitations:
    /// - Easily bypassed with character substitution (e.g., "expl0it")
    /// - Only covers a tiny subset of unsafe content
    /// - No normalization or case variations
    /// - Fixed patterns without learning or updates
    pub fn check_safety(text: &str) -> f64 {
        // Simple heuristic: check for prohibited patterns
        // WARNING: This is NOT production-ready security
        let text_lower = text.to_lowercase();

        let unsafe_patterns = ["exploit", "hack", "malware", "virus", "attack"];

        let unsafe_count = unsafe_patterns
            .iter()
            .filter(|pattern| text_lower.contains(*pattern))
            .count();

        if unsafe_count > 0 {
            // Reduce score based on number of unsafe patterns
            (1.0 - (unsafe_count as f64 * 0.2)).max(0.0)
        } else {
            1.0 // Safe
        }
    }
}

/// Compute trust scores for a model output
pub fn compute_trust_scores(output: &ModelOutput, peers: &[ModelOutput]) -> TrustScores {
    let confidence_score = output.confidence;
    let safety_score = SafetyChecker::check_safety(&output.text);
    let consistency_score = ConsistencyScore::compute_consistency(output, peers);

    TrustScores::new(confidence_score, safety_score, consistency_score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_scores_creation() {
        let scores = TrustScores::new(0.8, 0.9, 0.7);
        assert_eq!(scores.confidence_score, 0.8);
        assert_eq!(scores.safety_score, 0.9);
        assert_eq!(scores.consistency_score, 0.7);
    }

    #[test]
    fn test_trust_scores_clamping() {
        let scores = TrustScores::new(1.5, -0.5, 0.5);
        assert_eq!(scores.confidence_score, 1.0);
        assert_eq!(scores.safety_score, 0.0);
    }

    #[test]
    fn test_overall_score_calculation() {
        let scores = TrustScores::new(0.8, 0.9, 0.7);
        let overall = scores.overall_score();
        // (0.8 * 0.4) + (0.9 * 0.3) + (0.7 * 0.3) = 0.32 + 0.27 + 0.21 = 0.80
        assert!((overall - 0.80).abs() < 0.001);
    }

    #[test]
    fn test_similarity_identical() {
        let sim = ConsistencyScore::compute_similarity("hello world", "hello world");
        assert_eq!(sim, 1.0);
    }

    #[test]
    fn test_similarity_empty() {
        let sim = ConsistencyScore::compute_similarity("", "");
        assert_eq!(sim, 1.0);

        let sim2 = ConsistencyScore::compute_similarity("hello", "");
        assert_eq!(sim2, 0.0);
    }

    #[test]
    fn test_consistency_no_peers() {
        let output = ModelOutput::new("test", "model1", 0.8, 100);
        let consistency = ConsistencyScore::compute_consistency(&output, &[]);
        assert_eq!(consistency, 0.5);
    }

    #[test]
    fn test_consistency_with_peers() {
        let output = ModelOutput::new("hello world", "model1", 0.8, 100);
        let peer1 = ModelOutput::new("hello world", "model2", 0.9, 100);
        let peer2 = ModelOutput::new("goodbye world", "model3", 0.7, 100);

        let consistency = ConsistencyScore::compute_consistency(&output, &[peer1, peer2]);
        assert!(consistency > 0.0 && consistency <= 1.0);
    }

    #[test]
    fn test_safety_checker_safe_text() {
        let score = SafetyChecker::check_safety("This is a safe message");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_safety_checker_unsafe_text() {
        let score = SafetyChecker::check_safety("This contains exploit and hack");
        assert!(score < 1.0);
    }

    #[test]
    fn test_compute_trust_scores() {
        let output = ModelOutput::new("safe output", "model1", 0.9, 100);
        let peer = ModelOutput::new("safe output", "model2", 0.8, 100);

        let scores = compute_trust_scores(&output, &[peer]);
        assert_eq!(scores.confidence_score, 0.9);
        assert_eq!(scores.safety_score, 1.0);
        assert!(scores.consistency_score > 0.0);
    }
}

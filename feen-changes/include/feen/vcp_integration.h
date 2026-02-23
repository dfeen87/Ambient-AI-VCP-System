#ifndef FEEN_VCP_INTEGRATION_H
#define FEEN_VCP_INTEGRATION_H

#include <vector>
#include <map>
#include <string>

/**
 * FEEN VCP Integration Header
 * ---------------------------
 * Provides a minimal C++ interface for embedding FEEN as a native capability
 * within Ambient AI VCP's WASM or native execution environments.
 */

namespace feen {
namespace vcp {

    struct ResonatorConfig {
        double frequency_hz;
        double q_factor;
        double beta;
    };

    struct ResonatorState {
        double x;
        double v;
        double energy;
        double phase;
    };

    struct Excitation {
        double amplitude;
        double frequency_hz;
        double phase;
    };

    struct AileeSample {
        double p_input;
        double workload;
        double velocity;
        double inertia;
        double dt;
    };

    /**
     * Stateless simulation step for a single resonator.
     * Can be called from VCP's WASM engine via FFI.
     */
    ResonatorState simulate_resonator(const ResonatorConfig& config,
                                      const ResonatorState& current_state,
                                      const Excitation& input,
                                      double dt,
                                      unsigned int steps = 1);

    /**
     * Compute the AILEE Delta-v metric for a sequence of samples.
     * This allows VCP to verify efficiency metrics locally.
     */
    double compute_delta_v(const std::vector<AileeSample>& samples,
                           double alpha,
                           double v0,
                           double isp,
                           double eta);

} // namespace vcp
} // namespace feen

#endif // FEEN_VCP_INTEGRATION_H

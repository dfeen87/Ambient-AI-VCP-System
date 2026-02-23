from flask import Blueprint, request, jsonify
from feen.vcp import simulate_resonator, update_coupling, compute_delta_v

vcp_blueprint = Blueprint('vcp', __name__)

@vcp_blueprint.route('/api/v1/simulate', methods=['POST'])
def simulate():
    """
    Stateless simulation endpoint for VCP integration.
    Expects:
    {
        "config": { "frequency_hz": float, "q_factor": float, "beta": float },
        "state": { "x": float, "v": float, "energy": float, "phase": float },
        "input": { "amplitude": float, "frequency_hz": float, "phase": float },
        "dt": float,
        "steps": int (optional)
    }
    """
    data = request.get_json()

    config = data.get('config')
    state = data.get('state')
    input_signal = data.get('input')
    dt = data.get('dt')
    steps = data.get('steps', 1)

    if not all([config, state, input_signal, dt]):
        return jsonify({"error": "Missing required fields"}), 400

    try:
        new_state = simulate_resonator(config, state, input_signal, dt, steps)
        return jsonify({"state": new_state})
    except Exception as e:
        return jsonify({"error": str(e)}), 500

@vcp_blueprint.route('/api/v1/delta_v', methods=['POST'])
def calculate_delta_v():
    """
    Compute Delta-v for a sequence of samples.
    Expects:
    {
        "samples": [{ "p_input": float, "workload": float, "velocity": float, "inertia": float, "dt": float }, ...],
        "params": { "isp": float, "eta": float, "alpha": float, "v0": float }
    }
    """
    data = request.get_json()
    samples = data.get('samples', [])
    params = data.get('params', {})

    result = compute_delta_v(samples, params)
    return jsonify({"delta_v": result})

@vcp_blueprint.route('/api/v1/coupling', methods=['POST'])
def update_coupling_endpoint():
    """
    Update coupling configuration dynamically.
    Expects:
    {
        "source_id": str,
        "target_id": str,
        "strength": float,
        "phase_shift": float
    }
    """
    data = request.get_json()

    # In a real integration, this would call into the engine to update the network graph
    try:
        # Assuming we have access to the network context or it's stateless
        # Here we just validate and call the core update function
        new_config = update_coupling({}, data) # Mock empty network config for now
        return jsonify({"status": "success", "config": new_config})
    except Exception as e:
        return jsonify({"error": str(e)}), 500

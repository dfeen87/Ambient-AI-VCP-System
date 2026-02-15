// This is the ZK circuit.
// It proves that the prover knows a private input "x"
// that, when multiplied by itself, equals a public input "y".

pragma circom 2.0.0;

template SqrtProof() {
    // Private Input (the secret answer)
    signal input x;

    // Public Input (the question)
    signal input y;

    // The logic: we constrain x*x to equal y.
    // If x*x does not equal y, the proof will fail to generate.
    signal xSquared;
    xSquared <== x * x;
    y === xSquared;
}

// We instantiate the template
component main {public [y]} = SqrtProof();

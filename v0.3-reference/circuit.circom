// This is the ZK circuit.
// It proves that the prover knows a private input "x"
// that, when multiplied by itself, equals a public input "y".

pragma circom 2.0.0;

template SqrtProof() {
    // Private Input (the secret answer)
    signal input x;

    // Public Input (the question)
    signal output y;

    // The logic: we constrain x*x to equal y.
    // If x*x does not equal y, the proof will fail to generate.
    x * x === y;
}

// We instantiate the template
component main = SqrtProof();

/*
 * =================================================================
 * ZK-PROOF SETUP (RUN THIS SCRIPT ONCE)
 * =================================================================
 * This script does all the one-time "trusted setup" for our
 * ZK-Proof system. It:
 *
 * 1. Compiles the `circuit.circom` file.
 * 2. Performs a "trusted setup" (a.k.a. "powers of tau").
 * 3. Generates the `proving_key.zkey` (for Workers).
 * 4. Generates the `verification_key.json` (for Clients/Ledgers).
 *
 * You only need to run this one time!
 * `npm install`
 * `npm install -g circom` (if you don't have it)
 * `node setup.js`
 * =================================================================
 */

import { execSync } from 'child_process';
import { existsSync, mkdirSync, writeFileSync } from 'fs';

async function setupZk() {
  console.log('Starting ZK-Proof setup...');

  // 1. Create a directory for our ZK files if it doesn't exist
  const zkDir = './zk';
  if (!existsSync(zkDir)) {
    mkdirSync(zkDir);
    console.log('Created ./zk directory.');
  }

  try {
    // 2. Compile the circuit
    console.log('[1/5] Compiling circuit.circom...');
    execSync('circom circuit.circom --r1cs --wasm --sym -o zk', { stdio: 'inherit' });
    console.log('✅ Circuit compiled.');

    // 3. Start the "Powers of Tau" trusted setup
    // We'll use a small one for testing
    console.log('[2/5] Starting Powers of Tau ceremony (pt 1)...');
    if (!existsSync('./zk/pot12_0000.ptau')) {
      execSync('snarkjs powersoftau new bn128 12 ./zk/pot12_0000.ptau -v', { stdio: 'inherit' });
    }
    console.log('✅ Powers of Tau pt 1 complete.');

    console.log('[3/5] Contributing to Powers of Tau ceremony (pt 2)...');
    execSync('snarkjs powersoftau contribute ./zk/pot12_0000.ptau ./zk/pot12_0001.ptau --name="First Contribution" -v', { stdio: 'inherit' });
    console.log('✅ Powers of Tau pt 2 complete.');

    // 4. Generate circuit-specific keys
    console.log('[4/5] Generating circuit-specific proving key...');
    execSync('snarkjs plonk setup ./zk/circuit.r1cs ./zk/pot12_0001.ptau ./zk/proving_key.zkey', { stdio: 'inherit' });
    console.log('✅ Proving key generated: ./zk/proving_key.zkey');

    // 5. Export the verification key
    console.log('[5/5] Exporting verification key...');
    execSync('snarkjs zkey export verificationkey ./zk/proving_key.zkey ./zk/verification_key.json', { stdio: 'inherit' });
    
    console.log('✅ Verification key exported: ./zk/verification_key.json');
    console.log('\n==================================');
    console.log('🎉 ZK-Proof Setup Complete! 🎉');
    console.log('==================================');

  } catch (err) {
    console.error('❌ ZK-Proof Setup FAILED:', err.message);
  }
}

setupZk();

/*
 * =================================================================
 * AMBIENT AI NODE (v0.3 - ZK WORKER)
 * =================================================================
 * This node now generates ZK-Proofs instead of running TensorFlow.
 *
 * It will:
 * 1. Listen for "TASK:compute:y=..."
 * 2. Privately calculate the answer (the square root).
 * 3. Use `snarkjs` and `proving_key.zkey` to generate a proof.
 * 4. Publish the "RESULT:proof={...}" to the network.
 * =================================================================
 */

import { createLibp2p } from 'libp2p'
import { tcp } from '@libp2p/tcp'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'
import { mdns } from '@libp2p/mdns'
import { gossipsub } from '@libp2p/gossipsub'
import { fromString as uint8ArrayFromString } from 'uint8arrays/from-string'
import { toString as uint8ArrayToString } from 'uint8arrays/to-string'
import { plonk } from 'snarkjs'; // <-- IMPORT SNARKJS
import { readFileSync } from 'fs'; // <-- To read our keys
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

// Helper to get directory name in ES Modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const AMBIENT_AI_TOPIC = 'ambient-ai-v1'
let currentNodeStatus = 'idle'
let node;

// --- Load ZK Keys ---
const provingKeyPath = join(__dirname, 'zk', 'proving_key.zkey');
let PROVING_KEY;
try {
  PROVING_KEY = readFileSync(provingKeyPath);
  console.log('[ZK]: Proving key loaded successfully.');
} catch (err) {
  console.error('======================================================');
  console.error('❌ FATAL ERROR: Could not read ./zk/proving_key.zkey');
  console.error('Did you run `node setup.js` first?');
  console.error('======================================================');
  process.exit(1);
}

/**
 * --- The "Work" ---
 * This is our ZK-Proof generation.
 * It proves we know "x" (the private input) for "y" (the public input).
 */
async function runZKWork(taskMessage) {
  currentNodeStatus = 'working';
  console.log(`[WORK]: ZK Task received! Setting status to "working".`);
  await publishMessage(`STATUS:working`);

  try {
    // 1. Parse the task
    // e.g., TASK:compute:y=49:reward=1:task_id=...:client=...
    const parts = taskMessage.split(':');
    const y_str = parts.find(p => p.startsWith('y='))?.split('=')[1];
    const taskId = parts.find(p => p.startsWith('task_id='))?.split('=')[1];
    const publicInput_y = parseInt(y_str);
    
    // 2. Calculate the private input "x"
    const privateInput_x = Math.sqrt(publicInput_y);
    console.log(`[WORK]: Public input (y) is ${publicInput_y}. Private input (x) is ${privateInput_x}.`);

    // 3. Define inputs for the circuit
    const inputs = {
      x: privateInput_x,
      y: publicInput_y
    };
    
    // 4. Generate the ZK-Proof
    console.log('[WORK]: Generating ZK-Proof... (this may take a second)');
    const { proof, publicSignals } = await plonk.fullProve(
      inputs, 
      './zk/circuit.wasm', 
      PROVING_KEY
    );
    console.log('[WORK]: ✅ Proof generated!');

    // 5. Publish the result (the proof and public signals)
    const resultMessage = `RESULT:proof=${JSON.stringify(proof)}:publicSignals=${JSON.stringify(publicSignals)}:task_id=${taskId}:worker=${node.peerId.toString()}`;
    await publishMessage(resultMessage);
  
  } catch (err) {
    console.error('[ZK ERROR]:', err);
  }

  currentNodeStatus = 'idle';
  console.log('[WORK]: Task finished. Setting status back to "idle".');
}

/**
 * Helper function to publish messages
 */
async function publishMessage(message) {
  try {
    await node.services.pubsub.publish(AMBIENT_AI_TOPIC, uint8ArrayFromString(message));
  } catch (err) {
    console.error('[ERROR]: Could not publish message', err);
  }
}

const main = async () => {
  console.log('Starting Ambient AI ZK-Worker Node (v0.3)...');

  node = await createLibp2p({
    transports: [tcp()],
    streamMuxers: [mplex()],
    connectionEncryption: [noise()],
    peerDiscovery: [mdns()],
    services: { pubsub: gossipsub({ allowPublishToZeroPeers: true }) }
  });

  await node.start();
  console.log('✅ Node Started!');
  console.log('=================================================================');
  console.log('My Peer ID is:', node.peerId.toString());
  console.log('=================================================================');
  console.log('This is a ZK-WORKER node. Waiting for tasks...');

  node.services.pubsub.subscribe(AMBIENT_AI_TOPIC);

  node.services.pubsub.addEventListener('message', (evt) => {
    const from = evt.detail.from.toString();
    const message = uint8ArrayToString(evt.detail.data);

    if (from === node.peerId.toString()) return;

    if (message.startsWith('TASK:compute') && currentNodeStatus === 'idle') {
      runZKWork(message);
    } else if (message.startsWith('LEDGER:')) {
      console.log(`[LEDGER UPDATE]: ${message}`);
    }
  });
};

main();

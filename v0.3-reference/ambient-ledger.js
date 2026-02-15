/*
 * =================================================================
 * AMBIENT AI LEDGER (v0.3 - "THE ZK SMART CONTRACT")
 * =================================================================
 * This is the "Smart Contract" node. It:
 * 1. Listens for new "TASK" messages and stores them.
 * 2. Listens for "RESULT" messages containing a ZK-Proof.
 * 3. Verifies the proof using `verification_key.json`.
 * 4. If the proof is valid, it *automatically* pays the worker
 * from the client's account.
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

// --- Our Simple "Blockchain" ---
const ledger = {};
const openTasks = {}; // Store open tasks

let node;

// --- Load ZK Keys ---
const verificationKeyPath = join(__dirname, 'zk', 'verification_key.json');
let VERIFICATION_KEY;
try {
  VERIFICATION_KEY = JSON.parse(readFileSync(verificationKeyPath, 'utf8'));
  console.log('[ZK]: Verification key loaded successfully.');
} catch (err) {
  console.error('======================================================');
  console.error('❌ FATAL ERROR: Could not read ./zk/verification_key.json');
  console.error('Did you run `node setup.js` first?');
  console.error('======================================================');
  process.exit(1);
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

/**
 * Get the balance for a peer, or initialize it.
 */
function getBalance(peerId) {
  if (!ledger[peerId]) {
    ledger[peerId] = 100; // All new peers start with 100 demo tokens
  }
  return ledger[peerId];
}

/**
 * Process a Task and store it
 */
function processNewTask(taskMessage) {
  try {
    // e.g., TASK:compute:y=49:reward=1:task_id=...:client=...
    const parts = taskMessage.split(':');
    const y = parts.find(p => p.startsWith('y='))?.split('=')[1];
    const reward = parseInt(parts.find(p => p.startsWith('reward='))?.split('=')[1]);
    const taskId = parts.find(p => p.startsWith('task_id='))?.split('=')[1];
    const client = parts.find(p => p.startsWith('client='))?.split('=')[1];

    if (!y || !reward || !taskId || !client) return;
    
    // Check if client has funds
    if (getBalance(client) < reward) {
        console.warn(`[LEDGER]: Client ${client.slice(0,6)} has insufficient funds to post task.`);
        return;
    }

    // "Escrow" the funds
    ledger[client] -= reward;
    
    openTasks[taskId] = { y: parseInt(y), reward, client, status: 'open' };
    console.log(`[LEDGER]: New task ${taskId} posted by ${client.slice(0, 6)}. Reward of ${reward} is in escrow.`);

  } catch (err) {
    console.error('[LEDGER ERROR]: Failed to process new task', err);
  }
}

/**
 * Process a Result, Verify the Proof, and Pay
 */
async function processResult(resultMessage) {
  try {
    // e.g., RESULT:proof={...}:publicSignals=["49"]:task_id=...:worker=...
    
    // This string parsing is fragile, but works for our demo
    const proofStrStart = resultMessage.indexOf('proof=') + 6;
    const proofStrEnd = resultMessage.indexOf(':publicSignals=');
    const proofStr = resultMessage.substring(proofStrStart, proofStrEnd);

    const publicSignalsStart = resultMessage.indexOf('publicSignals=') + 14;
    const publicSignalsEnd = resultMessage.indexOf(':task_id=');
    const publicSignalsStr = resultMessage.substring(publicSignalsStart, publicSignalsEnd);
    
    const taskIdStart = resultMessage.indexOf('task_id=') + 8;
    const taskIdEnd = resultMessage.indexOf(':worker=');
    const taskId = resultMessage.substring(taskIdStart, taskIdEnd);

    const workerId = resultMessage.split(':worker=')[1];


    const proof = JSON.parse(proofStr);
    const publicSignals = JSON.parse(publicSignalsStr);

    if (!proof || !publicSignals || !taskId || !workerId) {
      console.error('[LEDGER ERROR]: Invalid result message');
      return;
    }

    // 1. Find the matching task
    const task = openTasks[taskId];
    if (!task || task.status !== 'open') {
      console.warn(`[LEDGER]: Received a result for an unknown or closed task: ${taskId}`);
      return;
    }

    console.log(`[LEDGER]: Received proof for task ${taskId} from worker ${workerId.slice(0, 6)}...`);

    // 2. Check if the public signals match the task
    // Our circuit's public output "y" is the first public signal
    const publicInput_y = publicSignals[0];
    if (publicInput_y != task.y) {
      console.error(`[LEDGER]: ❌ ZK-PROOF FAILED! Public input mismatch. Task was ${task.y}, proof was for ${publicInput_y}`);
      return;
    }

    // 3. **VERIFY THE PROOF**
    console.log('[LEDGER]: Verifying ZK-Proof...');
    const isVerified = await plonk.verify(VERIFICATION_KEY, publicSignals, proof);

    if (isVerified) {
      console.log(`[LEDGER]: ✅ ZK-PROOF VERIFIED! Paying worker...`);
      // 4. Pay the worker
      getBalance(workerId); // Initialize worker account if it's new
      ledger[workerId] += task.reward;
      
      // Mark task as paid
      task.status = 'paid';
      
      // 5. Publish the good news
      const successMsg = `LEDGER:payment_success:task_id=${taskId}:worker=${workerId.slice(0, 6)}:amount=${task.reward}`;
      await publishMessage(successMsg);
      
    } else {
      console.error(`[LEDGER]: ❌ ZK-PROOF FAILED! Proof is invalid for task ${taskId}.`);
      // Return funds to client
      ledger[task.client] += task.reward;
      task.status = 'failed';
    }

  } catch (err) {
    console.error(`[LEDGER ERROR]: Failed to process result: ${err.message}`);
    // A common error is a malformed JSON string from the pubsub message.
    // In a real system, this would need more robust parsing.
  }
}

const main = async () => {
  console.log('Starting Ambient AI Ledger Node (v0.3 ZK Verifier)...');
  node = await createLibp2p({
    transports: [tcp()],
    streamMuxers: [mplex()],
    connectionEncryption: [noise()],
    peerDiscovery: [mdns()],
    services: { pubsub: gossipsub({ allowPublishToZeroPeers: true }) }
  });

  await node.start();
  console.log('✅ Ledger Node Started!');
  console.log(`My Peer ID is: ${node.peerId.toString()}`);
  console.log('This is the ZK-LEDGER node. Waiting for tasks and proofs...');

  node.services.pubsub.subscribe(AMBIENT_AI_TOPIC);

  node.services.pubsub.addEventListener('message', (evt) => {
    const from = evt.detail.from.toString();
    const message = uint8ArrayToString(evt.detail.data);

    if (from === node.peerId.toString()) return;

    if (message.startsWith('TASK:compute')) {
      processNewTask(message);
    } else if (message.startsWith('RESULT:proof')) {
      processResult(message);
    }
  });

  // Publish the Ledger State every 20 seconds
  setInterval(() => {
    const ledgerState = `LEDGER:state:${JSON.stringify(ledger)}`;
    console.log('[LEDGER]: Publishing current ledger state...');
    publishMessage(ledgerState);
  }, 20000);
};

main();

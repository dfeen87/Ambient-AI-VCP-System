/*
 * =================================================================
 * AMBIENT AI CLIENT (v0.3 - ZK REQUESTER)
 * =================================================================
 * This node now just publishes a task with a reward.
 * It waits for the Ledger to confirm payment.
 * It no longer trusts or verifies the worker's result directly.
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

const AMBIENT_AI_TOPIC = 'ambient-ai-v1'
let node;
let tasksSent = 0;
let tasksPaid = 0;
let currentTaskId = null;

async function publishMessage(message) {
  try {
    await node.services.pubsub.publish(AMBIENT_AI_TOPIC, uint8ArrayFromString(message));
  } catch (err) {
    console.error('[ERROR]: Could not publish message', err);
  }
}

const main = async () => {
  console.log('Starting Ambient AI Client Node (v0.3)...');
  node = await createLibp2p({
    transports: [tcp()],
    streamMuxers: [mplex()],
    connectionEncryption: [noise()],
    peerDiscovery: [mdns()],
    services: { pubsub: gossipsub({ allowPublishToZeroPeers: true }) }
  });

  await node.start();
  console.log('✅ Client Node Started!');
  console.log(`My Peer ID is: ${node.peerId.toString()}`);
  console.log('=================================================================');
  console.log('This is a ZK-REQUESTER node. Will send tasks...');

  node.services.pubsub.subscribe(AMBIENT_AI_TOPIC);

  node.services.pubsub.addEventListener('message', (evt) => {
    const from = evt.detail.from.toString();
    const message = uint8ArrayToString(evt.detail.data);

    if (from === node.peerId.toString()) return;

    if (message.startsWith('LEDGER:payment_success')) {
      // We got confirmation!
      const parts = message.split(':');
      const paidTaskId = parts.find(p => p.startsWith('task_id='))?.split('=')[1];
      
      if (paidTaskId === currentTaskId) {
        console.log(`[CLIENT]: 🎉 Payment CONFIRMED for task ${paidTaskId}! Ready for next task.`);
        tasksPaid++;
        currentTaskId = null; // Ready for a new task
      }
    } else if (message.startsWith('STATUS:')) {
      console.log(`[NETWORK]: Worker status update: ${message}`);
    }
  });

  // Publish a new task every 15 seconds
  setInterval(() => {
    // Only send a new task if the last one was paid
    if (currentTaskId !== null) {
      console.log('[CLIENT]: Waiting for last task to be completed and paid...');
      return;
    }

    tasksSent++;
    currentTaskId = `task_id_${tasksSent}`;
    
    // This is our "work". We ask for the square root of 49.
    const publicInput = 49;
    const reward = 1;

    const taskMessage = `TASK:compute:y=${publicInput}:reward=${reward}:task_id=${currentTaskId}:client=${node.peerId.toString()}`;
    
    console.log(`[CLIENT]: Publishing new ZK task: "Find x where x*x = ${publicInput}" for ${reward} token.`);
    publishMessage(taskMessage);
    
  }, 15000); // 15 seconds
};

main();

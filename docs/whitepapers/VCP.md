The Verifiable Computation Protocol (VCP) – Ambient AI SDK: The Code Foundation (MVF): From Proof-of-Compute to a Universal Fabric

Don Michael Feeney Jr

November 4, 2025


1. Abstract
This document presents the v1.0 Technical Specification for the Verifiable Computation Protocol (VCP), a production-ready architecture evolving from the v0.3-alpha "Proof-of-Compute" engine.

The v0.3-alpha codebase—comprising ambient-node.js, ambient-client.js, ambient-ledger.js, and circuit.circom—successfully demonstrated a trustless, three-actor economic loop using libp2p and snarkjs.

v1.0 builds directly upon this foundation, introducing technically advanced components that elevate VCP from a single-purpose computation (e.g., x*x=y) to a universal, privacy-preserving, free Internet compute fabric. Key innovations include:

Secure Compute Sandbox: A Wasm-based runtime replacing the hard-coded runZKWork, enabling arbitrary, user-submitted code execution.
Universal ZK-Verifier: A ZK-ML and execution-trace-based proof system replacing the SqrtProof circuit, enabling verifiable AI inference and complex computations.
Federated Learning Protocol: A Layer-2 protocol leveraging VCP’s base layer for privacy-preserving, multi-node model training.
Intelligent Orchestrator: A reputation-based scheduler evolving the network from pubsub broadcasts to an optimized, game-theoretic compute market.

2. Analysis of the v0.3-alpha "Proof-of-Compute"
The v0.3-alpha codebase successfully established the “Architecture of Truth”, achieving atomic settlement: a Client’s funds are held in escrow by the Ledger and released to a Worker only if a valid ZK-Proof is submitted.

Two deliberate limitations in v0.3 have been resolved in v1.0:

Fixed Computation: runZKWork was hard-coded to a single function in circuit.circom.
Fixed Verification: Proofs were tied to a single equation (x*x=y) and could not verify AI model inference.

v1.0 systematically replaces both components with general-purpose, scalable solutions.

3. v1.0 Technical Specification: Secure Compute Sandbox (WasmEngine)
The Secure Compute Sandbox replaces the v0.3 hard-coded computation with a general-purpose, secure execution environment. This becomes the core of every VCP Worker Node.

v0.3 Worker Logic (simplified):

Receive TASK:compute:y=49.
Call runZKWork(49).
Generate proof.

v1.0 Worker Logic:

Receive Task: TASK:compute with a JSON payload.
Fetch & Cache: Download the Wasm module (e.g., model_inference.wasm) via P2P (IPFS) using compute_hash.
Instantiate Sandbox: Launch an isolated WasmEngine runtime (e.g., WasmEdge, Wasmer).
Confinement: Sandbox has zero permissions—no filesystem, network, or system access; only inputs are exposed.
Execute: Call the designated function (e.g., run_inference) with inputs.
Get Result: Receive INFERENCE_RESULT (e.g., [0.98, "cat"]).
Destroy Sandbox: Immediately terminate the Wasm instance.

This flow transforms the Worker from a “calculator” into a secure, general-purpose cloud function, fulfilling the vision of the Ambient Compute Mesh.

4. v1.0 Technical Specification: Universal ZK-Verifier (ZK-ML)
The Universal ZK-Verifier replaces the v0.3 simple ZK circuit. With arbitrary Wasm programs, proofs must now demonstrate Proof-of-Execution:

“I, the Worker, executed the Wasm program compute_hash correctly with these inputs, producing this INFERENCE_RESULT.”
v0.3 Proof Generation (simplified):

inputs = { x: 7, y: 49 }
plonk.fullProve(inputs, "circuit.wasm", "proving_key.zkey") 
v1.0 Proof Generation:

Run WasmEngine in proving mode.
Record execution trace (every opcode and memory change).
Feed trace into a Universal Prover (e.g., ZK-VM like RISC Zero, or VCP’s ZK-ML framework).
Generate PROOF_OF_EXECUTION.
Publish:

{
  "task_id": "...",
  "worker_id": "...",
  "public_result": [0.98, "cat"],
  "proof_of_execution": {...}
} 
Ledger nodes use Universal Verification Keys to verify proofs, cryptographically linking compute_hash, inputs, and results—realizing “Proof Replaces Trust.”

5. v1.0 Protocol Specification: Federated Learning
The VCP v1.0 Layer-2 protocol enables privacy-preserving, multi-node model training.

Actors:

Aggregator (Client): Requests model training.
Participants (Workers): Train the model on private local data.
Ledger (Verifier): Verifies training proofs and manages token settlement.

FL Protocol Loop:

Job Post & Escrow: Client publishes TASK:federated_learning with global_model_hash, training_data_hash, reward, and participant count.
Local Training & Proof: Workers train locally, generate Proof-of-Correct-Training attesting correct execution without revealing private data.
Verification & Aggregation: Aggregator/Ledger verifies all proofs. Aggregator aggregates verified model deltas (ΔM_1 + ΔM_2 + …) → new global model.
Settlement: Ledger releases escrowed tokens to verified Workers.

This achieves a decentralized, privacy-preserving, incentivized system for collective intelligence.

6. v1.0 Protocol Specification: Intelligent Orchestrator
The Intelligent Orchestrator evolves v0.3 pubsub broadcasts into an optimized, reputation-driven compute market.

Components:

Reputation Ledger: 

NODE_REPUTATION = Task_Success_Count / (Task_Success_Count + Task_Failure_Count)
Network Metrics: Real-time node latency map.
Scheduling Algorithm: Assigns tasks to the highest-scoring Workers rather than broadcasting, incentivizing speed, reliability, and honesty.

This transforms VCP into a game-theoretic, self-optimizing compute ecosystem.

7. Conclusion
The v0.3-alpha codebase proved that a trustless economic loop is possible.

VCP v1.0 delivers the architectural components to transform this proof-of-concept into the Ambient AI Infrastructure:

WasmEngine: Arbitrary compute in a secure sandbox.
Universal ZK-Verifier: AI-scale, verifiable proofs.
Federated Learning Protocol: Collective intelligence without data leakage.
Intelligent Orchestrator: Optimized, reputation-based compute market.

Together, these innovations establish a free, self-governing, universally accessible compute fabric, fully realizing the VCP vision.

Visualizer 
This code creates a high-level, interactive simulation of your entire Ambient AI Infrastructure. It visually demonstrates the "macro" view of your economy: idle devices (green nodes) being dynamically assigned by an "Orchestrator" to pending AI tasks (purple squares), and then simulates the token payments as those tasks are completed. It's the perfect way to show the emergent behavior of the compute mesh at scale.

<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Ambient AI - Visual Demo</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <style>
        /* Using Tailwind's 'Inter' font */
        body {
            font-family: 'Inter', sans-serif;
        }
        canvas {
            border: 1px solid #374151; /* gray-700 */
        }
    </style>
</head>
<body class="bg-gray-900 text-white p-4 lg:p-8 flex flex-col min-h-screen">
    <header class="text-center mb-6">
        <h1 class="text-3xl lg:text-5xl font-bold text-blue-400">Ambient AI Infrastructure</h1>
        <p class="text-lg lg:text-xl text-gray-300">Visual Simulation of the Decentralized Compute Mesh</p>
    </header>

    <!-- Main content area -->
    <main class="flex-grow flex flex-col">
        <!-- Canvas for simulation -->
        <div class="bg-gray-800 rounded-lg shadow-2xl overflow-hidden flex-grow">
            <canvas id="ambientCanvas"></canvas>
        </div>

        <!-- Control Panel -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-6">
            <!-- Stats Panel -->
            <div class="md:col-span-2 bg-gray-800 p-4 rounded-lg shadow-lg flex justify-around items-center flex-wrap">
                <div class="text-center m-2">
                    <span class="text-xs uppercase text-gray-400">Total Devices</span>
                    <p id="node-count" class="text-3xl font-bold text-blue-400">0</p>
                </div>
                <div class="text-center m-2">
                    <span class="text-xs uppercase text-gray-400">Idle Devices</span>
                    <p id="idle-count" class="text-3xl font-bold text-green-400">0</p>
                </div>
                <div class="text-center m-2">
                    <span class="text-xs uppercase text-gray-400">Working Devices</span>
                    <p id="working-count" class="text-3xl font-bold text-yellow-400">0</p>
                </div>
                <div class="text-center m-2">
                    <span class="text-xs uppercase text-gray-400">Tokens Paid</span>
                    <p id="token-count" class="text-3xl font-bold text-purple-400">0</scrip>
                </div>
            </div>

            <!-- Action Buttons -->
            <div class="bg-gray-800 p-4 rounded-lg shadow-lg flex flex-col md:flex-row justify-center items-center space-y-2 md:space-y-0 md:space-x-4">
                <button id="addNode" class="w-full md:w-auto bg-blue-600 hover:bg-blue-500 text-white font-bold py-3 px-6 rounded-lg shadow-md transition duration-200">
                    + Add Device
                </button>
                <button id="addTask" class="w-full md:w-auto bg-purple-600 hover:bg-purple-500 text-white font-bold py-3 px-6 rounded-lg shadow-md transition duration-200">
                    + Submit AI Task
                </button>
            </div>
        </div>
    </main>

    <script>
        const canvas = document.getElementById('ambientCanvas');
        const ctx = canvas.getContext('2d');
        let nodes = [];
        let tasks = [];
        let totalTokensPaid = 0;

        // Stats elements
        const nodeCountEl = document.getElementById('node-count');
        const idleCountEl = document.getElementById('idle-count');
        const workingCountEl = document.getElementById('working-count');
        const tokenCountEl = document.getElementById('token-count');

        // --- Resize canvas to fit its container ---
        function resizeCanvas() {
            canvas.width = canvas.parentElement.clientWidth;
            canvas.height = Math.max(400, window.innerHeight * 0.5); // 50% of viewport height, min 400px
        }
        window.addEventListener('resize', resizeCanvas);
        resizeCanvas();

        // --- Node Class ---
        class Node {
            constructor() {
                this.x = Math.random() * canvas.width;
                this.y = Math.random() * canvas.height;
                this.radius = 10;
                this.status = 'idle'; // 'idle', 'working'
                this.tokenBalance = 0;
                this.vx = (Math.random() - 0.5) * 0.5; // Slow drift
                this.vy = (Math.random() - 0.5) * 0.5; // Slow drift
                this.target = null; // Target task
            }

            draw() {
                ctx.beginPath();
                ctx.arc(this.x, this.y, this.radius, 0, Math.PI * 2);
                
                // Color based on status
                if (this.status === 'idle') {
                    ctx.fillStyle = 'rgba(52, 211, 153, 0.8)'; // green-400
                    ctx.strokeStyle = 'rgba(16, 185, 129, 1)'; // green-600
                } else if (this.status === 'working') {
                    ctx.fillStyle = 'rgba(250, 204, 21, 0.8)'; // yellow-400
                    ctx.strokeStyle = 'rgba(217, 119, 6, 1)'; // yellow-600
                }
                
                ctx.fill();
                ctx.lineWidth = 2;
                ctx.stroke();
                ctx.closePath();

                // Draw line to task if working
                if (this.status === 'working' && this.target) {
                    ctx.beginPath();
                    ctx.moveTo(this.x, this.y);
                    ctx.lineTo(this.target.x, this.target.y);
                    ctx.strokeStyle = 'rgba(250, 204, 21, 0.3)'; // yellow-400/30
                    ctx.lineWidth = 1;
                    ctx.stroke();
                    ctx.closePath();
                }
            }

            update() {
                // Drifting movement
                this.x += this.vx;
                this.y += this.vy;

                // Wall collision
                if (this.x < this.radius || this.x > canvas.width - this.radius) this.vx *= -1;
                if (this.y < this.radius || this.y > canvas.height - this.radius) this.vy *= -1;

                this.draw();
            }
        }

        // --- Task Class ---
        class Task {
            constructor() {
                this.id = Math.random();
                this.x = canvas.width / 2;
                this.y = canvas.height / 2;
                this.size = 20;
                this.status = 'pending'; // 'pending', 'processing', 'complete'
                this.workNeeded = 5; // Needs 5 nodes
                this.nodes = [];
            }

            draw() {
                ctx.beginPath();
                let color = 'rgba(129, 140, 248, 0.8)'; // indigo-400
                if (this.status === 'processing') {
                    color = 'rgba(250, 204, 21, 0.8)'; // yellow-400
                } else if (this.status === 'complete') {
                    color = 'rgba(167, 139, 250, 0.8)'; // purple-400
                }
                
                ctx.fillStyle = color;
                ctx.strokeStyle = 'white';
                ctx.lineWidth = 2;
                ctx.rect(this.x - this.size / 2, this.y - this.size / 2, this.size, this.size);
                ctx.fill();
                ctx.stroke();
                ctx.closePath();
            }
        }

        // --- Orchestrator Logic (The "AI Scheduler") ---
        function orchestrate() {
            const pendingTask = tasks.find(t => t.status === 'pending');
            if (!pendingTask) return;

            const idleNodes = nodes.filter(n => n.status === 'idle');
            
            if (idleNodes.length >= pendingTask.workNeeded) {
                // We have enough nodes! Assign the work.
                console.log("Orchestrator: Found idle nodes! Assigning task...");
                pendingTask.status = 'processing';
                
                for (let i = 0; i < pendingTask.workNeeded; i++) {
                    const node = idleNodes[i];
                    node.status = 'working';
                    node.target = pendingTask;
                    pendingTask.nodes.push(node);
                }

                // Simulate work being done (Federated Learning / ZK-Proof generation)
                setTimeout(() => {
                    completeTask(pendingTask);
                }, 3000); // 3 seconds of "work"
            }
        }

        // --- Task Completion & Payment ---
        function completeTask(task) {
            console.log("Orchestrator: Task complete! Paying nodes.");
            task.status = 'complete';
            
            // Pay nodes and set them back to idle
            task.nodes.forEach(node => {
                node.status = 'idle';
                node.target = null;
                node.tokenBalance += 1; // Pay 1 token
                totalTokensPaid += 1;
            });

            // Remove task from array after a short delay
            setTimeout(() => {
                tasks = tasks.filter(t => t.id !== task.id);
            }, 1000);
        }

        // --- Update Stats Panel ---
        function updateStats() {
            let idle = 0;
            let working = 0;
            nodes.forEach(n => {
                if (n.status === 'idle') idle++;
                if (n.status === 'working') working++;
            });

            nodeCountEl.textContent = nodes.length;
            idleCountEl.textContent = idle;
            workingCountEl.textContent = working;
            tokenCountEl.textContent = totalTokensPaid;
        }

        // --- Main Animation Loop ---
        function animate() {
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            // Run orchestrator logic
            orchestrate();

            // Update and draw all elements
            tasks.forEach(task => task.draw());
            nodes.forEach(node => node.update());

            // Update the HTML stats
            updateStats();

            requestAnimationFrame(animate);
        }

        // --- Button Event Listeners ---
        document.getElementById('addNode').addEventListener('click', () => {
            nodes.push(new Node());
            console.log(`Added node. Total: ${nodes.length}`);
        });

        document.getElementById('addTask').addEventListener('click', () => {
            // Only add one task at a time for this demo
            if (tasks.filter(t => t.status !== 'complete').length === 0) {
                tasks.push(new Task());
                console.log("Submitted new AI task.");
            } else {
                console.log("Waiting for current task to complete.");
                // We could show a visual message here instead of just logging
            }
        });

        // Start the simulation
        animate();
    </script>
</body>
</html>
 NPM Dependencies
This package.json file is the technical manifest for the v0.3-alpha protocol, defining the core communication and networking stack. Its importance lies in the dependencies list, which is built entirely around libp2p and its modules (gossipsub, mdns, noise, tcp). This isn't just a list of libraries; it is the concrete proof that the Ambient AI Infrastructure is not a theoretical, cloud-based system. It confirms the protocol is built from the ground up on a modular, peer-to-peer, and serverless foundation, providing the exact tools needed to create the "self-organizing," "censorship-resistant," and "decentralized" mesh network described in the original vision.

{
  "name": "ambient-ai-node",
  "version": "0.1.0",
  "description": "Foundational node for the Ambient AI Infrastructure",
  "main": "ambient-node.js",
  "type": "module",
  "scripts": {
    "start": "node ambient-node.js"
  },
  "dependencies": {
    "@libp2p/bootstrap": "^1.1.1",
    "@libp2p/gossipsub": "^10.0.1",
    "@libp2p/mdns": "^1.1.1",
    "@libp2p/mplex": "^1.0.1",
    "@libp2p/noise": "^15.0.0",
    "@libp2p/tcp": "^1.0.1",
    "libp2p": "^1.3.1"
  }
}
Ambient Node & Client 
This code snippet is the foundational "engine" of the Ambient AI SDK, representing the original communication and discovery layer of the VCP. It defines the two primary actors in the decentralized economy: the Ambient Node (Worker), which connects to the libp2p mesh and periodically broadcasts its availability (STATUS:idle), and the Ambient Client (Requester), which broadcasts its need for computation (TASK:compute). Its importance is that it solves the first and most critical problem: creating the serverless, self-organizing "Ambient Compute Mesh" where supply and demand can meet. This simple, decentralized gossip protocol is the technical "how" behind the marketplace, creating the open P2P fabric upon which all subsequent layers of verification, payment, and orchestration (like the v0.3 ZK-Ledger) are built.

*
 * =================================================================
 * AMBIENT AI NODE (v0.1)
 * =================================================================
 * This is the foundational, runnable code for a single node in the
 * Ambient AI Infrastructure.
 *
 * This node will:
 * 1. Create a unique, cryptographic Peer ID.
 * 2. Connect to the libp2p network using TCP and mDNS for local discovery.
 * 3. Join a 'pubsub' (Gossipsub) topic to communicate with other nodes.
 * 4. Announce its "idle" status every 10 seconds.
 * 5. Listen for and print messages from other nodes.
 *
 * This is the "engine" for the "Ambient SDK".
 * =================================================================
 */

// We use ES Modules (import) - note the "type": "module" in package.json
import { createLibp2p } from 'libp2p'
import { tcp } from '@libp2p/tcp'
import { mplex } from '@libp2p/mplex'
import { noise } from '@chainsafe/libp2p-noise'
import { mdns } from '@libp2p/mdns'
import { gossipsub } from '@libp2p/gossipsub'
import { fromString as uint8ArrayFromString } from 'uint8arrays/from-string'
import { toString as uint8ArrayToString } from 'uint8arrays/to-string'

// --- The Topic for our AI Network ---
// This is the "channel" all our nodes will listen to.
const AMBIENT_AI_TOPIC = 'ambient-ai-discovery-v1'

const main = async () => {
  console.log('Starting Ambient AI Node...')

  // --- 1. Create the Node ---
  // This creates a new libp2p node with all the services we need.
  const node = await createLibp2p({
    // A node's address is its unique PeerId
    // This will be generated for us and printed to the console.
    transports: [
      tcp() // We'll use TCP for transport
    ],
    streamMuxers: [
      mplex() // For handling multiple data streams over one connection
    ],
    connectionEncryption: [
      noise() // For secure, encrypted connections
    ],
    peerDiscovery: [
      mdns()  // For discovering other nodes on the *local* network (like your Wi-Fi)
    ],
    services: {
      // We're adding Gossipsub, the pubsub protocol
      pubsub: gossipsub({
        allowPublishToZeroPeers: true // Allows us to publish even if we're the first node
      })
    }
  })

  // --- 2. Start the Node ---
  await node.start()
  console.log('✅ Node Started!')
  console.log('=================================================================')
  console.log('My Peer ID is:', node.peerId.toString())
  console.log('Listening on addresses:')
  node.getMultiaddrs().forEach((addr) => {
    console.log(addr.toString())
  })
  console.log('=================================================================')

  // --- 3. Listen for other nodes ---
  node.addEventListener('peer:discovery', (evt) => {
    const peer = evt.detail
    console.log(`[NETWORK]: Discovered peer: ${peer.id.toString()}`)
  })

  node.addEventListener('peer:connect', (evt) => {
    const peer = evt.detail
    console.log(`[NETWORK]: Connected to peer: ${peer.toString()}`)
  })

  // --- 4. Join the Ambient AI Topic ---
  node.services.pubsub.subscribe(AMBIENT_AI_TOPIC)

  // Listen for messages from other nodes on this topic
  node.services.pubsub.addEventListener('message', (evt) => {
    const from = evt.detail.from.toString()
    const message = uint8ArrayToString(evt.detail.data)

    // Don't print our own messages
    if (from === node.peerId.toString()) {
      return
    }

    console.log(`[MESSAGE RECEIVED from ${from.slice(0, 6)}...]: ${message}`)
  })

  // --- 5. Announce Our Status to the Network ---
  // This is the "heartbeat" of our node.
  // Every 10 seconds, it announces it is "idle" and ready for work.
  setInterval(() => {
    const message = `STATUS:idle:CAP=low:TYPE=cpu` // A simple message
    
    // Publish this message to the topic
    node.services.pubsub.publish(AMBIENT_AI_TOPIC, uint8ArrayFromString(message))
      .then(() => {
        console.log('[MESSAGE SENT]: Announcing "idle" status...')
      })
      .catch((err) => {
        console.error('[ERROR]: Could not publish message', err)
      })
  }, 10000) // 10 seconds
}

// Run the main function
main()/*
 * =================================================================
 * AMBIENT AI CLIENT (v0.1 - REQUESTER)
 * =================================================================
 * This is a "Requester" node. It does NOT do work.
 *
 * It will:
 * 1. Connect to the libp2p network.
 * 2. Every 15 seconds, it will "REQUEST" a task by
 * publishing a "TASK:compute" message.
 * 3. It will listen for "STATUS:" and "RESULT:" messages
 * from the Worker Nodes.
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

// --- The Topic for our AI Network ---
const AMBIENT_AI_TOPIC = 'ambient-ai-v1'

const main = async () => {
  console.log('Starting Ambient AI Client Node (Requester)...')

  const node = await createLibp2p({
    transports: [tcp()],
    streamMuxers: [mplex()],
    connectionEncryption: [noise()],
    peerDiscovery: [mdns()],
    services: {
      pubsub: gossipsub({ allowPublishToZeroPeers: true })
    }
  })

  // --- Start the Node ---
  await node.start()
  console.log('✅ Client Node Started!')
  console.log(`My Peer ID is: ${node.peerId.toString()}`)
  console.log('This is a REQUESTER node. Will send tasks...')

  // --- Join the Topic ---
  node.services.pubsub.subscribe(AMBIENT_AI_TOPIC)

  // --- Listen for responses from workers ---
  node.services.pubsub.addEventListener('message', (evt) => {
    const from = evt.detail.from.toString()
    const message = uint8ArrayToString(evt.detail.data)

    if (from === node.peerId.toString()) return

    if (message.startsWith('STATUS:')) {
      console.log(`[WORKER STATUS from ${from.slice(0, 6)}...]: ${message}`)
    } else if (message.startsWith('RESULT:')) {
      console.log(`[!!! AI RESULT RECEIVED from ${from.slice(0, 6)}...]: ${message}`)
    }
  })

  // --- Publish a new task every 15 seconds ---
  setInterval(() => {
    const taskMessage = `TASK:compute:model=simple-regression:input=10`
    
    console.log('[CLIENT]: Publishing new AI task to the network...')
    
    node.services.pubsub.publish(
      AMBIENT_AI_TOPIC,
      uint8ArrayFromString(taskMessage)
    ).catch(err => {
      console.error('[ERROR]: Could not publish task', err)
    })
  }, 15000) // 15 seconds
}

main()
Ambient AI Ledger
This code snippet represents the foundational "engine" of the Ambient AI SDK, demonstrating the first and most critical layer of the entire system: the peer-to-peer communication and discovery fabric. It defines the two primary actors in the decentralized economy: the Ambient Node (Worker), which connects to the libp2p mesh and periodically broadcasts its availability (STATUS:idle), and the Ambient Client (Requester), which broadcasts its need for computation (TASK:compute). This simple, serverless, and self-organizing gossip protocol is the technical "how" behind the "Ambient Compute Mesh" vision, creating the open marketplace for compute upon which all subsequent layers of verification, payment, and orchestration are built.

/*
 * =================================================================
 * AMBIENT AI LEDGER (v0.1 - "THE SMART CONTRACT")
 * =================================================================
 * This is a special "Validator" node. Its only job is to
 * listen for "MSG:pay" commands and update the central ledger.
 *
 * It will:
 * 1. Connect to the libp2p network.
 * 2. Maintain a simple JSON object for account balances.
 * 3. Listen for "MSG:pay:from=...:to=...:amount=..."
 * 4. On a valid payment message, it updates the balances.
 * 5. Every 30 seconds, it publishes the entire ledger state.
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

// --- The Topic for our AI Network ---
const AMBIENT_AI_TOPIC = 'ambient-ai-v1'

// --- Our Simple "Blockchain" ---
const ledger = {
  // We will add peer IDs here as they transact
  // e.g., "12D3Koo...": 100
}

let node;

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
    // New accounts start with 100 tokens (for clients)
    // or 0 (for workers)
    // For this demo, we'll just give clients 100 to start.
    // A real system would have a "faucet" or "on-ramp".
    // We'll assume clients have money and workers don't.
    ledger[peerId] = peerId.startsWith('CLIENT') ? 100 : 0; // Just a demo placeholder
  }
  return ledger[peerId];
}

/**
 * Process a payment message
 */
function processPayment(message) {
  try {
    const parts = message.split(':');
    const fromId = parts.find(p => p.startsWith('from='))?.split('=')[1];
    const toId = parts.find(p => p.startsWith('to='))?.split('=')[1];
    const amountStr = parts.find(p => p.startsWith('amount='))?.split('=')[1];
    const amount = parseInt(amountStr);

    if (!fromId || !toId || !amount) {
      console.error('[LEDGER ERROR]: Invalid payment message:', message);
      return;
    }

    // Initialize balances if they don't exist
    // In our simple demo, we'll assume the client has infinite money
    // A real system would check: if (getBalance(fromId) < amount) return;
    
    // For this demo, we'll just give the client a high starting balance
    if (!ledger[fromId]) ledger[fromId] = 1000;
    if (!ledger[toId]) ledger[toId] = 0;


    // Check if client has enough funds
    if (ledger[fromId] < amount) {
        console.warn(`[LEDGER]: Payment failed. ${fromId.slice(0, 6)} has insufficient funds.`);
        return;
    }

    // Transfer funds
    ledger[fromId] -= amount;
    ledger[toId] += amount;

    console.log(`[LEDGER]: Payment SUCCESS. ${amount} tokens from ${fromId.slice(0, 6)} to ${toId.slice(0, 6)}`);

  } catch (err) {
    console.error('[LEDGER ERROR]: Failed to process payment', err);
  }
}


const main = async () => {
  console.log('Starting Ambient AI Ledger Node (v0.1)...')

  node = await createLibp2p({
    transports: [tcp()],
    streamMuxers: [mplex()],
    connectionEncryption: [noise()],
    peerDiscovery: [mdns()],
    services: {
      pubsub: gossipsub({ allowPublishToZeroPeers: true })
    }
  })

  // --- Start the Node ---
  await node.start()
  console.log('✅ Ledger Node Started!')
  console.log(`My Peer ID is: ${node.peerId.toString()}`)
  console.log('This is the LEDGER node. Listening for payments...')

  // --- Join the Topic ---
  node.services.pubsub.subscribe(AMBIENT_AI_TOPIC)

  // --- Listen for messages ---
  node.services.pubsub.addEventListener('message', (evt) => {
    const from = evt.detail.from.toString()
    const message = uint8ArrayToString(evt.detail.data)

    if (from === node.peerId.toString()) return

    // --- This is the core logic ---
    if (message.startsWith('MSG:pay')) {
      // We got a payment command!
      processPayment(message);
    }
  })

  // --- Publish the Ledger State ---
  // Every 20 seconds, broadcast the state of all accounts
  setInterval(() => {
    const ledgerState = `LEDGER:state:${JSON.stringify(ledger)}`
    console.log('[LEDGER]: Publishing current ledger state...');
    publishMessage(ledgerState);
  }, 20000) // 20 seconds
}

main()
ZK-Proof Circuit
This circuit.circom code represents the technical heart of the v0.3-alpha "Proof-of-Compute" engine, defining the verifiable contract for the work to be done. In this foundational SqrtProof template, the circuit proves a simple, specific algebraic statement: that a Worker (the prover) knows a private secret x that, when squared, equals a public task y.

The significance of this circuit for the paper is not the mathematical problem itself, but its critical role as a verifiable stand-in for arbitrary computation. This simple logic is the first "V" in the Verifiable Compute Protocol. It is the component that allows the autonomous Ledger node to cryptographically validate a Worker's result without re-running the work or trusting the Worker. This circuit proves the entire trustless economic loop (Task -> Escrow -> Compute -> Prove -> Verify -> Settle) is viable, establishing the solid architectural foundation that v1.0 will expand upon by replacing this simple component with a universal ZK-ML verifier.

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
ZK-Proof Setup (RUN ONCE)
This setup.js script represents the critical, one-time "key generation ceremony" for the v0.3-alpha protocol. Its primary significance is that it forges the two asymmetric, cryptographic keys that enable the entire trustless economic loop, translating the abstract logic of circuit.circom into a practical, verifiable system. By compiling the circuit and running a trusted setup, it produces the proving_key.zkey—the tool that empowers any Worker to generate a valid proof of their computation. Simultaneously, it creates the verification_key.json, which is the tool that gives the autonomous Ledger node the power to verify that proof, all without re-running the work or trusting the worker. This script is the bridge from abstract logic to a deployable "Architecture of Truth," establishing the verifiable, asymmetric roles that allow the network to settle payments.

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
    const verificationKeyJson = execSync('snarkjs zkey export verificationkey ./zk/proving_key.zkey ./zk/verification_key.json').toString();
    // We need to parse this and re-save it for easy import
    const vKey = JSON.parse(verificationKeyJson);
    writeFileSync('./zk/verification_key.json', JSON.stringify(vKey, null, 2));
    
    console.log('✅ Verification key exported: ./zk/verification_key.json');
    console.log('\n==================================');
    console.log('🎉 ZK-Proof Setup Complete! 🎉');
    console.log('==================================');

  } catch (err) {
    console.error('❌ ZK-Proof Setup FAILED:', err.message);
  }
}

setupZk();
package.json (The Dependencies)
This package.json file is the definitive technical blueprint for the v1.0 SDK, and its importance lies in the synthesis of its dependencies. It explicitly shows how the protocol merges three distinct, powerful technology stacks into one unified system. First, the libp2p modules confirm the serverless, peer-to-peer Networking Layer (the "Ambient Compute Mesh"). Second, snarkjs and circomlibjs provide the Verifiable Trust Layer for generating and verifying all cryptographic "Proofs-of-Compute." Finally, the inclusion of @tensorflow/tfjs-node is the most critical new element, representing the AI Execution Layer. This dependency is the first concrete, technical step that bridges the v0.3 "Proof-of-Logic" (x*x=y) with the advanced v1.0 vision of running real, verifiable AI workloads, making it a cornerstone of this paper's advanced technical specification.

{
  "name": "ambient-ai-node",
  "version": "1.0.0",
  "description": "Ambient AI SDK Node",
  "main": "ambient-node.js",
  "type": "module",
  "scripts": {
    "start": "node ambient-node.js",
    "client": "node ambient-client.js",
    "ledger": "node ambient-ledger.js",
    "zk-setup": "node setup.js"
  },
  "dependencies": {
    "@libp2p/bootstrap": "^1.1.1",
    "@libp2p/gossipsub": "^10.0.1",
    "@libp2p/mdns": "^1.1.1",
    "@libp2p/mplex": "^1.0.1",
    "@libp2p/noise": "^15.0.0",
    "@libp2p/tcp": "^1.0.1",
    "libp2p": "^1.3.1",
    "@tensorflow/tfjs-node": "^4.20.0",
    "circomlibjs": "^0.1.7",
    "snarkjs": "^0.7.4"
  }
} 
circuit.circom (The ZK-Proof Logic)
This circuit.circom code is the technical heart of the v0.3-alpha "Proof-of-Compute" engine, defining the verifiable contract for the work to be done. In this foundational SqrtProof template, the circuit proves a simple, specific algebraic statement: that a Worker (the prover) knows a private secret x that, when squared, equals a public task y.

The significance of this circuit for the paper is not the mathematical problem itself, but its critical role as a verifiable stand-in for arbitrary computation. This simple logic is the first "V" in the Verifiable Compute Protocol. It is the component that allows the autonomous Ledger node to cryptographically validate a Worker's result without re-running the work or trusting the Worker. This circuit proves the entire trustless economic loop (Task -> Escrow -> Compute -> Prove -> Verify -> SSettle) is viable, establishing the solid architectural foundation that v1.0 will expand upon by replacing this simple component with a universal ZK-ML verifier.

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
setup.js (The One-Time Key Generator)
This setup.js script represents the critical, one-time "key generation ceremony" for the ZK-Proof system. Its importance in this paper is that it forges the two asymmetric, cryptographic keys that enable the entire trustless economic loop, translating the abstract logic of circuit.circom into a practical, verifiable system. By compiling the circuit and running a trusted setup, it produces the proving_key.zkey—the tool that empowers any Worker to generate a valid proof of their computation. Simultaneously, it creates the verification_key.json, which is the tool that gives the autonomous Ledger node the power to verify that proof, all without re-running the work or trusting the worker. This script is the bridge from abstract logic to a deployable "Architecture of Truth," establishing the verifiable, asymmetric roles that allow the network to settle payments.

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
    const verificationKeyJson = execSync('snarkjs zkey export verificationkey ./zk/proving_key.zkey ./zk/verification_key.json').toString();
    // We need to parse this and re-save it for easy import
    const vKey = JSON.parse(verificationKeyJson);
    writeFileSync('./zk/verification_key.json', JSON.stringify(vKey, null, 2));
    
    console.log('✅ Verification key exported: ./zk/verification_key.json');
    console.log('\n==================================');
    console.log('🎉 ZK-Proof Setup Complete! 🎉');
    console.log('==================================');

  } catch (err) {
    console.error('❌ ZK-Proof Setup FAILED:', err.message);
  }
}

setupZk(); 
ambient-client.js (The Job Requester)
This ambient-client.js file defines the first key actor in the v0.3 economic protocol: the Job Requester or Client. Its technical significance within this paper is how it perfectly demonstrates the "zero trust" principle of the compute marketplace. The node's logic is intentionally simple: it connects to the libp2p mesh and publishes a TASK:compute message, which includes the public inputs (y=49) and the economic incentive (reward=1). Critically, this client does not trust the worker. It doesn't listen for or validate the worker's RESULT message. Its only source of truth is the final, autonomous broadcast from the Ledger node: LEDGER:payment_success. By waiting for this cryptographic confirmation of settlement before posting its next job, this node provides the economic "demand" that drives the entire "Task -> Escrow -> Compute -> Prove -> Verify -> Settle" loop, confirming the system's trustless nature from the requester's side.

/*
 * =================================================================
 * AMBIENT AI CLIENT (v0.2 - ZK REQUESTER)
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
  console.log('Starting Ambient AI Client Node (v0.2)...');
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
ambient-node.js (The ZK Worker Node)
This ambient-node.js file defines the second key actor in the v0.3 economic protocol: the ZK-Worker or Compute Provider. This code represents the "supply" side of the decentralized compute market. Its logic is to listen for TASK:compute messages from Clients. When it accepts a job, it executes the runZKWork function: it privately calculates the answer (the "secret" x=7), then uses snarkjs and its unique proving_key.zkey to generate a cryptographic ZK-Proof that the computation was performed correctly.

Its importance to the VCP is that it never reveals its private work, only the proof. It perfectly embodies the "zero trust" model: it doesn't trust the Client to pay. It trusts the autonomous Ledger to cryptographically verify its RESULT:proof message and release the escrowed funds. This node is the technical embodiment of "verifiable work" and the "Proof-of-Compute" concept, providing the verifiable "supply" that the Client's "demand" pays for.

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
ambient-ledger.js (The "Smart Contract" Verifier)
This ambient-ledger.js file is the technical linchpin of the v0.3-alpha protocol, representing the autonomous "Smart Contract" and Verifier for the entire network. Its importance to this paper is that it is the "Architecture of Truth." It operates with zero trust, performing two critical, non-negotiable functions: first, it acts as the escrow agent by listening for TASK:compute messages and securing the Client's reward; second, it acts as the autonomous verifier in the processResult function. By loading the verification_key.json and executing plonk.verify, it cryptographically validates the Worker's ZK-Proof against the public task inputs. If the proof is valid, this node autonomously settles the payment and broadcasts the LEDGER:payment_success message, which serves as the only source of truth for both the Client and the Worker, officially completing the trustless economic loop.

/*
 * =================================================================
 * AMBIENT AI LEDGER (v0.2 - "THE ZK SMART CONTRACT")
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

  } catch (err)_ {
    console.error(`[LEDGER ERROR]: Failed to process result: ${err.message}`);
    // A common error is a malformed JSON string from the pubsub message.
    // In a real system, this would need more robust parsing.
  }
}

const main = async () => {
  console.log('Starting Ambient AI Ledger Node (v0.2 ZK Verifier)...');
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
Secure Compute Sandbox (WasmEngine) Logic
This EXECUTE_WASM_TASK_SECURELY pseudocode defines the core logic for the v1.0 Worker Node, representing a massive technical leap from the v0.3-alpha protocol. Its importance is that it replaces the hard-coded runZKWork function with a general-purpose, secure, and sandboxed execution environment. This logic directly solves the confinement problem outlined in the original white paper by specifying that all untrusted code (CODE_BYTE_ARRAY) must be run inside an isolated Wasm runtime (Steps 1-4), which has no access to the host's filesystem or network.

Furthermore, this function provides the critical cryptographic link for the next phase of the protocol. By hashing the executed code to produce a MODEL_HASH (Step 5), it generates the verifiable evidence that the Universal AI Verifier will use to prove that the correct AI model was executed. This pseudocode is the technical specification for graduating the network from proving a simple, fixed circuit (x*x=y) to proving the secure, verifiable execution of any user-submitted AI task, which is the foundational promise of the v1.0 architecture.

FUNCTION EXECUTE_WASM_TASK_SECURELY(CODE_BYTE_ARRAY, INPUT_DATA)
    // 1. ISOLATION: Create secure execution environment (simulated by Wasm VM)
    SANDBOX = CREATE_ISOLATED_WASM_RUNTIME()
    SANDBOX.MEMORY_LIMIT = 512MB
    SANDBOX.TIMEOUT = 30_SECONDS
    SANDBOX.BLOCK_ACCESS_TO('File_System', 'Network_Sockets', 'System_APIs')
    
    // 2. LOADING: Load the untrusted AI model/code
    LOAD_CODE(SANDBOX, CODE_BYTE_ARRAY)
    
    // 3. EXECUTION: Run the core inference function
    RESULT_VECTOR = CALL_FUNCTION_IN_SANDBOX(
        SANDBOX, 
        FUNCTION_NAME: "run_inference", 
        INPUTS: INPUT_DATA
    )
    
    // 4. CLEANUP: Destroy the sandbox instance immediately
    DESTROY(SANDBOX)
    
    // 5. ZK PREP: Hash the model weights (W) and inputs (D) for the ZK-Proof
    MODEL_HASH = HASH(CODE_BYTE_ARRAY) 
    
    RETURN { 
        OUTPUT: RESULT_VECTOR,
        MODEL_HASH: MODEL_HASH,
        SUCCESS: TRUE
    }
END FUNCTION
Universal AI Verifier (ZK-ML) Logic
This VERIFY_AI_INFERENCE pseudocode is the technical specification for the v1.0 Ledger Node, representing a significant evolution from the v0.3-alpha's simple processResult function. Its importance in this paper is that it details the new, multi-stage verification pipeline required for complex AI workloads. Instead of just a single cryptographic check, this function first verifies that the correct AI model was used by matching the MODEL_HASH from the proof against the original task (Step 2). This is a critical new security measure, preventing workers from using cheaper, fraudulent models.

Most importantly, this function explicitly defines the engine for the reputation system mentioned in the Intelligent Orchestrator logic. By directly calling UPDATE_REPUTATION—rewarding honest proofs (+1) and heavily penalizing fraudulent ones (-5)—this logic is no longer just a passive verifier. It becomes the active, game-theoretic "source of truth" that provides the immutable reputation data needed for the Orchestrator to build its WORKER_SCORE, making the entire self-optimizing, trust-based economy viable.

FUNCTION VERIFY_AI_INFERENCE(PROOF, PUBLIC_INPUTS, WORKER_ID)
    // 1. PUBLIC INPUTS: Check if the claimed model/input matches the task request
    TASK = GET_OPEN_TASK(PUBLIC_INPUTS.TASK_ID)
    
    IF TASK.EXPECTED_OUTPUT != PUBLIC_INPUTS.INFERENCE_RESULT THEN
        RETURN { VERIFIED: FALSE, REASON: "Output Mismatch" }
    
    // 2. VERIFIABLE MODEL: Check that the worker used the correct, approved AI model (W)
    IF TASK.MODEL_HASH != PUBLIC_INPUTS.MODEL_HASH THEN
        RETURN { VERIFIED: FALSE, REASON: "Untrusted Model Weights" }
    
    // 3. CRYPTOGRAPHIC VERIFICATION: The core ZK-SNARK verification function
    // This confirms that the worker ran the code correctly, without seeing the steps.
    IS_PROOF_VALID = CALL_ZK_SNARK_VERIFIER(
        VERIFICATION_KEY, 
        PROOF, 
        PUBLIC_INPUTS
    )
    
    IF IS_PROOF_VALID THEN
        CALL UPDATE_REPUTATION(WORKER_ID, +1) // REWARD HONESTY
        RETURN { VERIFIED: TRUE, RESULT: PUBLIC_INPUTS.INFERENCE_RESULT }
    ELSE
        CALL UPDATE_REPUTATION(WORKER_ID, -5) // PUNISH FRAUD HEAVILY
        RETURN { VERIFIED: FALSE, REASON: "Invalid Cryptographic Proof" }
END FUNCTION
Intelligent Orchestrator (Network Optimization) Logic
This pseudocode for the SCHEDULE_OPTIMAL_WORKER function is the technical specification for the Intelligent Orchestrator, a critical v1.0 component that evolves the network from a simple pubsub broadcast into an efficient, game-theoretic compute market. Its significance is that it defines how the network self-optimizes. Instead of a "first-come, first-serve" model, the Orchestrator calculates a WORKER_SCORE by weighting a node's historical NODE_REPUTATION (derived from the Ledger's immutable log of successful and failed proofs) against its real-time NODE_LATENCY. This "Game Theory Math" creates a powerful economic incentive for nodes to be reliable and fast, as the Orchestrator will directly assign high-value tasks to the BEST_WORKER, fulfilling the vision of a truly self-governing and optimized "Ambient Compute Mesh."

FUNCTION SCHEDULE_OPTIMAL_WORKER(TASK_PAYLOAD, AVAILABLE_NODES_LIST)
    BEST_SCORE = -INFINITY
    BEST_WORKER = NULL
    
    // 1. GATHER NETWORK METRICS: Query libp2p for real-time latency
    NETWORK_METRICS = QUERY_NETWORK_LATENCY(AVAILABLE_NODES_LIST)
    
    // 2. ITERATE: Score every available node
    FOR EACH NODE IN AVAILABLE_NODES_LIST DO
        // Retrieve trust history from the Ledger
        NODE_REPUTATION = GET_REPUTATION_SCORE(NODE.PEER_ID) 
        NODE_LATENCY = NETWORK_METRICS.LATENCY[NODE.PEER_ID]
        
        // 3. CALCULATE WEIGHTED SCORE (The Game Theory Math)
        // Score = (Reputation * Trust_Weight) - (Latency * Speed_Weight) + (Capacity * Power_Weight)
        // A strong worker (high Rep) with fast connection (low Latency) wins.
        TRUST_WEIGHT = 0.6  // Prioritize reliability
        SPEED_WEIGHT = 0.3  // Prioritize low latency
        
        WORKER_SCORE = (NODE_REPUTATION * TRUST_WEIGHT) - (NODE_LATENCY * SPEED_WEIGHT)
        
        // 4. SELECTION: Find the highest-scoring node
        IF WORKER_SCORE > BEST_SCORE THEN
            BEST_SCORE = WORKER_SCORE
            BEST_WORKER = NODE
        END IF
    END FOR
    
    IF BEST_WORKER != NULL THEN
        // 5. SEND TASK: Directly message the single best worker
        CALL SEND_MESSAGE_DIRECTLY(BEST_WORKER, TASK_PAYLOAD)
    ELSE
        // Fallback or broadcast to bootstrap the network
        CALL BROADCAST_TASK(TASK_PAYLOAD)
    END IF
END FUNCTION
An Example README.MD
The Ambient AI Infrastructure 
This is the foundational codebase for the Ambient AI Infrastructure, a decentralized, trustless, and incentivized compute network.

Our mission is to build the architecture for a Free Internet: a self-governing, uncensorable, and privacy-preserving digital commons built and owned by its participants. This project is the engine for that vision.

This codebase is not a "product" or a "company." It is an open-source protocol for a new, decentralized economy.

The Vision: The "Ambient AI" White Paper
This codebase is the first technical implementation of the principles described in our white paper, "Ambient AI Infrastructure: From Vision to Fabric."

We are building a system where:

Compute is Verifiable: We use Zero-Knowledge Proofs (ZK-SNARKs) to prove computation is done correctly without revealing the data.
Incentives are Automated: We use a decentralized, P2P ledger (a "smart contract") to automatically pay nodes for their provable work.
The Network is Resilient: We use a P2P mesh network (libp2p) so there is no central server, no single point of failure, and no "off" switch.

This is the engine for the "Ambient SDK."

See It Work: The v0.3 "Proof-of-Compute" Demo
This repository contains the v0.3 "Proof-of-Compute" engine. You can run it on your machine right now in three terminals to see the full, trustless economic loop.

1. Requirements
Node.js (v18+)
The circom compiler. Install it globally: npm install -g circom

2. Installation
Clone this repository and install the dependencies:

git clone <YOUR_REPO_URL_HERE>
cd ambient-ai-sdk
npm install 
3. One-Time Trusted Setup
We need to generate the ZK-Proof keys for our circuit. Run this command only once:

node setup.js 
This will compile the circuit.circom file and create a /zk folder containing your proving_key.zkey and verification_key.json.

4. Run the Network!
Open three (3) separate terminal windows in the project folder and run them in this order:

Terminal 1: The Ledger (The "Smart Contract")

node ambient-ledger.js 
Output you'll see: "✅ Ledger Node Started! ... Waiting for tasks and proofs..."

Terminal 2: The Worker (The "Compute Node")

node ambient-node.js 
Output you'll see: "✅ Node Started! ... Waiting for tasks..."

Terminal 3: The Client (The "Job Requester")

node ambient-client.js 
Output you'll see: "✅ Client Node Started! ... Publishing new ZK task..."

5. Watch the Magic
You will now see the "Architecture of Truth" in action across your terminals:

[Client] will publish: Publishing new ZK task: "Find x where x*x = 49"
[Ledger] will see it: New task task_id_1 posted... Reward of 1 is in escrow.
[Worker] will see it: ZK Task received! ... Generating ZK-Proof...
[Worker] will publish: Proof generated!
[Ledger] will see the proof: Received proof... Verifying ZK-Proof...
[Ledger] will confirm: ZK-PROOF VERIFIED! Paying worker...
[Client] will get confirmation: Payment CONFIRMED for task task_id_1!

You have just run a decentralized, trustless, and paid compute task.

How to Contribute (The 1-Year Mission)
This is the foundation. The 1-year mission is to build the full Ambient SDK from this engine.

We are actively funding development for the next modules through a decentralized bounty system. The roadmap is clear and we need your help.

Current Bounties & Next Steps:
[Bounty: AI-Integration]: Upgrade the ZK-Proof from x*x=y to a simple, verifiable AI model (e.g., proof of inference for a simple logistic regression).
[Bounty: WasmEdge-Runtime]: Replace the runZKWork function with a secure, sandboxed WasmEdge runtime.
[Bounty: Federated-Learning]: Create a new "Aggregator" node and implement a basic, privacy-preserving Federated Learning loop.
[Bounty: On-Chain-Ledger]: Replace the ambient-ledger.js node with real Solidity smart contracts on an L2 (e.g., Polygon, Arbitrum).

Join us in building a Free Internet.

Path to Adoption: The TON/Telegram Integration Strategy

To the Telegram Messenger and TON Foundation Teams,

We have long admired your work from a protocol-first perspective. While the world focused on building centralized, "walled-garden" communication apps, you built a resilient, decentralized, and peer-to-peer fabric for human communication. You successfully solved the problem of decentralized communication at a global scale.

We believe the next, and even more critical, frontier is decentralized computation. The entire AI revolution—the most powerful force in our world today—is currently being funneled through the same centralized, "black box," and rent-seeking cloud providers that your work was designed to bypass.

For the past several years, we have been building the missing piece of this puzzle. We call it the Verifiable Compute Protocol (VCP), an open-source, trustless, and incentivized compute network. We have enclosed our full technical white paper, which details the v1.0 architecture and includes the complete, runnable v0.3-alpha "Proof-of-Compute" engine.

Before you review the technicals, we must state our core intention. Our commitment to open knowledge and open-source principles is the indivisible core of this protocol; it is the only way to build a truly "Free Internet." The current digital world is built on proprietary, "black box" systems that centralize control, extract rent, and demand blind trust. This protocol is the antidote. By publishing every line of code, every cryptographic circuit, and every protocol specification as a public good, we are not just sharing software; we are open-sourcing the engine of a new, un-owned economy. This absolute transparency is the ultimate security, as it allows any participant to audit the "Architecture of Truth" and build upon it without asking permission. This is the mechanism that dismantles the walled gardens, guarantees a permissionless digital commons, and ensures the "unbranded" fabric of our future remains free.

This philosophy is precisely why we are approaching you. We are not a company seeking a partnership. We are protocol architects seeking alignment. While other platforms seek to build proprietary, closed ecosystems, you are the only team that has both the technical foundation (TON) and the user-scale (Telegram) that shares this core ethos.

We are asking you to "lead the charge" because VCP is not a competitor; it is a foundational, missing layer for your ecosystem. We have specifically designed the v1.0 architecture to bridge with TON:

Your Clients: Telegram Mini Apps (TMAs) are the perfect Clients. Your 900+ million users could submit complex AI or compute tasks directly from their chats.
Your Settlement Layer: The TON Blockchain is the ideal Settlement Layer. Our ambient-ledger.js can be re-implemented as a set of verification smart contracts on TON, using TON as the native currency for the entire compute economy.
Your Vision: This integration would create the world's first, and largest, verifiable decentralized compute market. It would transform Telegram from a communication fabric into a full-stack, decentralized operating system for the "Free Internet" we both envision.

We believe this is the next logical, technical, and philosophical step for The Open Network. We have attached the full white paper, which includes the v1.0 specifications for the Wasm-based Secure Sandbox, the ZK-ML Universal Verifier, and the Federated Learning protocol. The v0.3-alpha engine is working, and we would be honored to demonstrate it for your technical teams.

Thank you for your time and for building the tools that make this next layer possible.

Sincerely,

Don M. Feeney

The Following is a briefing on execution:

The VCP technical stack provides the "how," but a protocol's value is determined by its adoption. This section defines the v1.2 go-to-market strategy, which moves from a theoretical network to a practical, large-scale economy.

The Challenge: Protocol vs. Product A pure, open-source protocol (like our "un-owned, unbranded" vision) faces a "cold start" problem. It has no users. A private, for-profit product (like a "VCP, Inc.") could raise capital and market a service, but this would compromise the core philosophy of a decentralized "Free Internet" commons.

The Strategy: Protocol-First Integration Our strategy is not to build a new, private company, but to integrate the VCP protocol as a fundamental, missing layer for an existing, large-scale decentralized ecosystem.

Our primary integration target is TON (The Open Network), the decentralized protocol powering the Telegram messaging platform.

Why TON/Telegram?

Massive Adoption: Instant, addressable access to Telegram's 900+ million active users.
Philosophical Alignment: TON is a decentralized, open-source, and peer-to-peer ecosystem, perfectly matching the VCP vision.
Technical Synergy: Clients: Telegram Mini Apps (TMAs) can become the "Clients," allowing any Telegram user to request AI compute. Settlement: The TON blockchain is a live, high-speed settlement layer that can host our verifier. Payments: vcp tasks can be paid for using TON, grounding the protocol's economy in a real, liquid currency.

v1.2 Technical Roadmap: The VCP-TON Bridge

This integration strategy defines the next concrete development phase:

Ledger Becomes a Smart Contract: The ambient-ledger.js node will be re-implemented as a set of smart contracts (in FunC or Tact) deployed on the TON blockchain.
On-Chain Verification: The VERIFY_AI_INFERENCE logic (from the v1.0 ZK-ML spec) will be executed by this smart contract. The Universal Verification Key will be stored on-chain.
TMA as Client: The ambient-client.js logic will be adapted into a Telegram Mini App (TMA). A user inside Telegram can pay for a task, which publishes the TASK message to the VCP-TON contract.
Settlement Flow: A VCP Worker (running ambient-node.js) sees the TASK event from the TON contract. The Worker executes the task, generates the PROOF_OF_EXECUTION (as per the v1.0 spec). The Worker submits this PROOF to the VCP-TON smart contract. The contract's verify function runs. If it passes, the contract autonomously releases the TON tokens held in escrow to the Worker's wallet.

This strategy provides a clear, actionable path to mass adoption without compromising the core decentralized and "un-owned" principles of the Ambient AI Infrastructure.

Strategic Application: The VCP-Native AI Browser (e.g., OpenAI Atlas)
The current generation of "AI-assisted browsers" are simply API wrappers for centralized services. They offer convenience but inherit all the flaws of a centralized web: they are not trustless, not verifiable, and require users to surrender their data for processing. This is a stop-gap, not an evolution.

The true evolution of the browser is to become a verifiable, decentralized node in a broader digital economy. By integrating the Verifiable Compute Protocol (VCP) as a native component, the browser itself becomes the user's "trusted agent" in the "Free Internet" fabric.

This integration creates a new, synergistic layer where human, AI, and blockchain interactions converge into a single, auditable system.

The Browser as a VCP Client & Wallet
In this model, the browser is the vcp-client. It holds the user's keys, manages their VCP-based tokens, and acts as their "requester" on the decentralized mesh. This enables:

Trustless Computation for Users: A user visiting a decentralized application (dApp) no longer has to trust that dApp's remote server. When the user clicks "mint" or "submit," the dApp requests the browser to issue a TASK:compute to the VCP mesh. A VCP Worker, selected by the Intelligent Orchestrator, executes the task in its Wasm Sandbox. The ZK-Proof is generated and verified on-chain by the Ledger. The browser then receives the LEDGER:payment_success message and displays a "Cryptographically Verified" icon to the user. This is true "trust-by-design."

Decentralized, Privacy-Preserving AI Workflows
This is the "AI Companion for the Web." An AI agent built into the browser can now perform complex tasks without sacrificing user privacy.

Centralized (Current): A user asks the AI, "Summarize my browsing history for the last week." The browser must send that entire, highly-sensitive history to a central OpenAI/Google server, where it is logged, stored, and analyzed.
Decentralized (VCP): The user asks the same question. The browser's AI agent loads the history.db locally, then issues a TASK:compute to the VCP. This task includes a compute_hash for a "Summarizer" AI model and the private history data. A VCP Worker executes this task in its secure Wasm Sandbox, generates the summary, and generates a ZK-Proof attesting that it only ran the "Summarizer" model and did not (and could not) copy or leak the raw data. The browser gets the summary back with a cryptographic guarantee of privacy.

The Interface for an Open, Verifiable Value Economy
This is the direct link to a Phase 1 asset layer, a pattern first implemented by the BrightActs system. The browser becomes the primary interface for minting and managing "dignity-based" value from any participating dApp.

This architecture is open to all AI developers and creates a standard for "proof-of-impact":

A user performs a verified "goodwill action" on a dApp (e.g., writes a high-quality review, completes an open-source bounty).
The dApp requests the browser to submit this "proof-of-work" to the VCP.
The action is processed by an AI scoring model (like the one pioneered in BrightActs' "Metabolic System"), which runs as a verifiable VCP task.
A corresponding minting contract (like BrightActs' "Circulatory System") is then triggered, and the user's browser wallet receives the appropriate reputation or impact tokens (e.g., "LOVES," "BRIGHT," etc.).

This turns the VCP-native browser into a universal "value" layer, inviting any AI solution to create its own verifiable, tokenized economies on top of this shared, trustless infrastructure.

This approach positions the AI browser not just as a tool for information retrieval, but as the primary, high-trust interface for participating in a verifiable, decentralized, and impact-driven economy.

By bridging Phase 1 assets and Phase 2 protocol, this VCP-native AI browser creates a standard for verifiable computation and value creation. Builders, AI developers, and decentralized finance architects now have the opportunity to contribute to a Free Internet that is secure, auditable, and scalable. The infrastructure is here. The mission is clear. The future is verifiable.

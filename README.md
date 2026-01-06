# Fundamental Particle Simulation

A real-time, GPU-accelerated physics simulation of fundamental particles (quarks and electrons) interacting via the four fundamental forces. This project aims to visualize the emergence of complex structures—from hadronization (quarks forming protons/neutrons) to the formation of atomic nuclei and electron shells.

## 🌟 Features

### ⚛️ Physics Engine
*   **GPU-Accelerated N-Body Simulation:** Handles thousands of particles using `wgpu` compute shaders.
*   **Fundamental Forces:**
    *   **Strong Force:** Modeled with Color Charge dynamics and a Cornell potential (confinement + short-range freedom). Quarks dynamically bind into Baryons (Protons/Neutrons) and Mesons.
    *   **Electromagnetism:** Coulomb interaction driving electron orbits and proton repulsion.
    *   **Weak Force:** Short-range Yukawa potential.
    *   **Gravity:** Standard Newtonian attraction.
*   **Nucleon Physics:**
    *   **Residual Strong Force:** An effective Yukawa potential binds protons and neutrons into atomic nuclei.
    *   **Hadron Exclusion:** Hard-sphere repulsion prevents nucleons from merging into amorphous blobs.
    *   **Atomic Locking:** Ensures quarks are strictly assigned to unique hadrons.
*   **Electron Dynamics:**
    *   **Pauli-like Exclusion:** A repulsive force prevents electrons from collapsing into the nucleus, stabilizing atomic orbitals.

### 🎨 Visualization
*   **3D Rendering:** Instanced rendering for high-performance particle visualization.
*   **Hadron Shells:** Semi-transparent shells visualize the bounds of formed protons and neutrons.
*   **Internal Bonds:** Dynamic lines show the strong force connections between quarks.
*   **Real-time UI:** Built with `astra-gui` for interactive control.

## 🎮 Controls

### Camera
*   **Right Mouse Button + Drag:** Rotate camera around the center.
*   **Mouse Wheel:** Zoom in/out.

### Keyboard Shortcuts
*   **Space:** Pause / Resume simulation.
*   **Ctrl + Right Arrow / D:** Step forward (when paused).

### GUI Controls
The on-screen interface allows real-time tuning of the simulation:
*   **Time Controls:** Pause, resume, and step through the simulation frame-by-frame.
*   **Physics Parameters:** Tweak the strength and range of all forces (Gravity, Electric, Strong, Nucleon Binding, etc.) on the fly.
*   **Rendering Options:** Toggle the visibility of hadron shells and bonds.

## 🚀 Getting Started

### Prerequisites
*   **Rust:** Latest stable version.
*   **Vulkan/Metal/DX12:** A GPU compatible with `wgpu`.

### Running
```bash
cargo run --release
```
*Note: Release mode is highly recommended for performance.*

## 🧠 Physics Model Details

1.  **Quark Confinement:** Quarks carry Red, Green, or Blue color charge. The simulation enforces color neutrality, causing quarks to group into triplets (Baryons) or pairs (Mesons).
2.  **Nucleus Formation:** Once hadrons form, a secondary "Residual Strong Force" kicks in. This short-range attractive force overcomes the electromagnetic repulsion between protons, allowing stable nuclei to form.
3.  **Stability:** To prevent the simulation from exploding due to high-energy collisions, we implement velocity-dependent damping specifically for nucleon interactions, allowing them to settle into stable bound states.

## 🛠️ Tech Stack
*   **Language:** Rust
*   **Graphics API:** wgpu (WebGPU)
*   **UI:** astra-gui
*   **Math:** glam
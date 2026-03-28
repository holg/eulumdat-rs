# 🌌 Obscura Demo: Darkness Preservation Simulator

## 🎯 Objective
Create a high-performance, real-time lighting simulation in **Bevy (Rust)** that demonstrates the "scientific beauty of darkness." This demo showcases how the `eulumdat-bevy` crate handles real-world photometric data to reduce light pollution—perfect for a collaboration pitch to **L’Observatoire de la Nuit**.

---

## 🏗️ Architecture
- **Engine:** Bevy 0.18+ (ECS-based)
- **Core Plugin:** `eulumdat_bevy::photometric::PhotometricPlugin`
- **Primary Entity:** `EulumdatLightBundle`
- **Interactivity:** `bevy_egui` for a "Light Quality" dashboard

---

## 🛠️ Step 1: Initial Prompt for Claude Code
*Copy and paste this into your terminal running `claude`:*

> "Help me build a professional lighting demo for my `eulumdat-bevy` crate. We need to create an example called `obscura_demo`. Setup a 'Midnight Urban' environment: a dark grey ground plane (100x100), three minimalist 'apartment' cubes with emissive windows, and a 'park' area with a few spheres as trees. Ensure the ambient light is near zero to emphasize the photometric sources. Use `eulumdat_bevy::photometric::*` and `eulumdat_bevy::EulumdatLightBundle`."

---

## 🧪 Step 2: Scientific "Toggle" Logic
*Ask Claude to implement the comparison mode:*

> "Add a `SimulationState` with two modes: `StandardPollution` and `PreservedDarkness`. 
> 
> **In StandardPollution:** Spawn street lamps using a 'bad' LDT file (high intensity, 360° spread). Increase Bevy's `FogSettings` density to 0.05 (white) to simulate urban sky glow. 
> 
> **In PreservedDarkness:** Use a 'good' LDT file (shielded, directed downward). Set the `Color` to a warm 2700K. Decrease `FogSettings` to 0.005 and enable a procedural star-field background. 
> 
> Implement the `Space` key to toggle between these states instantly."

---

## 📊 Step 3: The "Obscura" Dashboard (Egui)
*Ask Claude to add the analytical UI:*

> "Integrate `bevy_egui`. Create a floating window titled 'Obscura Analysis'. Display:
> 1. **Current Mode:** (Standard vs. Preserved).
> 2. **LDT Filename:** Display the metadata from the active `EulumdatLight`.
> 3. **Environmental Impact:** Calculate a 'Sky Glow Score' based on the `Uplight` value from the LDT data.
> 4. **Live Control:** Add a slider to adjust the 'Atmospheric Haze' (Fog density) in real-time."

---

## 🎨 Step 4: Visual Polish & Performance
*Final prompt for the 'Wow' factor:*

> "Enhance the visuals: 
> 1. Enable **Bloom** and **Volumetric Lighting** on the photometric lights so we can see the 'cone' of light in the haze.
> 2. Add a **First-Person Fly Camera** (use `bevy_flycam` logic or a simple custom system).
> 3. Ensure we can spawn 100+ lights at 60fps+ to demonstrate the efficiency of the ECS architecture over Unity/Unreal alternatives."

---

## 🚀 Step 5: Run and Fix
*Tell Claude to verify:*

> "Run `cargo run --example obscura_demo`. If you encounter any dependency mismatches with Bevy 0.18 or issues with the `eulumdat` crate imports, fix them by inspecting the current workspace files."

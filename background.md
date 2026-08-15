Building a native, modern, streamlined RDP client tailored for the **COSMIC desktop environment** (written in Rust, utilizing libcosmic/iced toolkit) that cuts out decades of legacy baggage is a fantastic project. Remmina’s interface is cluttered because it has to support VNC, SSH, SPICE, SFTP, and XDMCP all at once, leading to modal dialogs buried inside modal dialogs.

Focusing strictly on an **RDP-only client optimized for Teams usage** (meaning solid audio redirection and clean scaling) allows for a massive simplification of the UI and stack.

### 1. Architectural Blueprint for a Modern COSMIC RDP Client

Instead of wrestling with a giant legacy C codebase like FreeRDP's full CLI or patching GTK widgets into Remmina, you can build a clean client using **Rust** and **libfreerdp bindings** wrapped in an **Iced / libcosmic** GUI.

#### Core UI Requirements (The "Anti-Remmina" Approach)

- **Zero Protocol Switchers:** Drop the protocol dropdown entirely. Every profile is an RDP profile.

- **Single-Window Connection Hub:** A clean grid of saved target Windows boxes (or saved `.rdp` files) with clear status badges.

- **Streamlined Settings Sheet:** Instead of a 5-tab settings monolith, condense everything essential for a Teams-heavy workflow into a single clean drawer or page:

- **Resolution/Display:** Native full screen vs. seamless window scaling.

- **Audio:** Toggle local microphone input (`audin`) and remote sound output (`rdpsnd`).

- **Camera:** Toggle camera passthrough routing (`rdpecam`).

- **Credentials:** Saved securely via the native system keyring (Freedesktop Secret Service / COSMIC credential store).

### 2. Tapping into FreeRDP from Rust

You don't need to write an RDP client parser from scratch—that would take years. Instead, leverage `libfreerdp` under the hood via Rust FFI or existing community bindings, but control the execution loop cleanly.

- **Client Core:** Initialize a headless FreeRDP client instance (`freerdp_client_context`), point it at your target IP, and pass the required command-line equivalent parameters programmatically:

- `/cert:ignore` (or proper thumbprint validation cache)

- `/audio-input` / `/microphone` for Teams mic injection

- `/dvc:rdpecam` or local video device mapping if handling camera streams

- **Display Integration:** FreeRDP can output its rendering frames via a localized Surface/Window handle. In a Rust GUI framework like Iced, you can render the incoming RDP graphics buffer directly into an active texture or canvas widget frame-by-frame.

### 3. Key Challenges to Anticipate for the Teams Use Case

1. **Microphone Latency & Sample Rates:** Windows RDP audio input (`audin`) defaults to specific sample rate constraints. Ensure your wrapper maps PipeWire/PulseAudio input cleanly to prevent Microsoft Teams from treating your remote mic input as distorted or lagging.

2. **Dynamic Resolution Scaling:** When you resize your client window on your COSMIC desktop, you'll want the remote Windows box to dynamically update its display resolution (`gfx` channel updates) so Teams doesn't render inside a tiny letterboxed rectangle. FreeRDP handles this via the Graphics Pipeline Extension (`EGFX`), which you can trigger on window resize events.

### Getting Started

Since COSMIC is built natively in Rust using the `iced` GUI library, bootstrapping a lightweight frontend using `libcosmic` will make your app look, feel, and perform like a first-party component of your desktop environment—sharing its theme, styling, and window behaviors seamlessly.

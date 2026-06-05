# OBS Studio Setup & Optimization Guide

This guide explains how to add the visualizer overlay into **OBS Studio**, optimize its performance, and customize its appearance using custom layout overrides.

---

## 1. Adding the Browser Source
To show the dynamic visualizer in OBS, we use a Browser Source.

1. Open **OBS Studio**.
2. Go to the **Sources** dock, click the **`+`** icon, and select **Browser**.
3. Name it something clear, like `Music Visualizer Overlay`.
4. In the properties window, configure the following settings:
   * **URL**: `http://localhost:3000` (Make sure the port matches your `settings.json`!).
   * **Width**: `1360` (Widescreen default)
   * **Height**: `440`
   * **Use Custom Frame Rate**: Checked (Set this to `60` or your target streaming FPS).
   * **Control Audio via OBS**: Unchecked (The overlay is visual-only; audio is captured natively by the backend).

---

## 2. Layout Viewport Customization
Depending on your streaming layout, you can load specific minimalist or compact configurations directly inside the OBS URL using **URL parameters**:

### Minimalist Coverless Layout:
Hides the track cover and expands text to fill the remaining screen space.
* **URL**: `http://localhost:3000?layout=compact-minimal`
* **Width**: `540`
* **Height**: `100`

### Compact Bar Layout:
A narrow horizontal strip with visualizer and text, perfect for sitting at the bottom of a webcam frame or stream border.
* **URL**: `http://localhost:3000?layout=compact-bar`
* **Width**: `800`
* **Height**: `200`

### Clean, No-Progress Overlay:
Hides the progress bar entirely (useful for live-stream clean aesthetics).
* **URL**: `http://localhost:3000?progress=false`

---

## 3. High Performance Optimization in OBS
Because the visualizer renders smooth audio spectrum animations, configuring OBS correctly guarantees buttery-smooth frames and saves GPU overhead:

1. **Enable Hardware Acceleration**:
   * Open OBS Studio **Settings** -> **Advanced**.
   * Under **Sources**, check **"Enable Browser Source Hardware Acceleration"**.
   * *Note*: If browser sources are lagging or stuttering, enabling this leverages your graphics card to render the animations at 60+ FPS cleanly.
2. **Auto-Sleep Setting**:
   * In your Browser Source properties, check **"Shutdown source when not visible"** and **"Refresh browser when scene becomes active"**.
   * This ensures that if you switch scenes (e.g. to a *Be Right Back* screen where the visualizer is not present), OBS completely suspends the overlay to save system resources.

---

## 4. Custom CSS Overlay Styling
You can inject custom CSS directly inside the Browser Source properties to match your stream’s aesthetic:

### Force Transparent Background:
The overlay is transparent by default, but if you want to force custom background opacity:
```css
body { background: transparent !important; }
#widget { background: rgba(0, 0, 0, 0.4) !important; backdrop-filter: blur(8px); }
```

### Stream Overlay Shadow:
Add a beautiful dark drop shadow to the entire widget so it stands out cleanly on top of high-detail gameplay:
```css
#widget {
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.8) !important;
}
```

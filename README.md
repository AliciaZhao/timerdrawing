# GermiBoard

Can't decide what to draw? Here we have a program that will help you decide! Using any folder which stores your references and art inspirations, GermiBoard will choose one for you!
GermiBoard will be able to stay on top while you draw away and even have a timer when you are active only in a certain program (So you can alt+tab and go do other business and the timer will pause for you).
This program is made using Rust and will be able to run even on your home microwave! Enjoy!

## Features

- Supports multiple image folders
- Right-click menu for navigation and tools
- Timer overlay that tracks how long an image has been shown
- Automatically resizes the window to fit each image
- Option to keep the window always on top
- Can pause the timer when a specific application is active (e.g., Photoshop)

## How to Use

1. Download or build the app.
2. Run `GermiBoard.exe` by double-clicking it.
3. Right-click anywhere in the window to:
   - Add image folders
   - Move to the next image
   - Toggle the timer and pin features
   - Track another application (like an EXE)

## Configuration
GermiBoard creates and uses a file called viewer_config.json:
This file stores:
- Folder selections
- The currently shown image index
- Whether always-on-top is enabled
- The name of a tracked EXE (if any)

You can delete this file to reset the app's settings.

## System Requirements

- Operating System: **Windows 10 or Windows 11 (64-bit)**
- CPU: **x86_64 (Intel or AMD, 64-bit architecture)**
- RAM: **4 GB or more recommended**
- GPU: **Integrated or dedicated GPU compatible with OpenGL or DirectX 11**
- Additional: **Visual C++ Redistributable (usually pre-installed)**

> No internet connection, installation, or administrative privileges are required to run the application.

## Compatibility

- GermiBoard is portable and does not require installation
- No internet access is required
- All application state is stored locally

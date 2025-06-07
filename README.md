# vrc-owo
OWO vest integration for VRChat

## Features
- Depth-dependent intensity
- Impact sensation for high velocities (can be disabled in the radial menu)
- Blade contact for various sword/dagger assets

If you have ideas for further improvements, feel free to let me know in an issue

## Installation
1. Download [vrc-owo.unitypackage](https://raw.githubusercontent.com/MrMineO5/vrc-owo/refs/heads/main/vrc-owo.unitypackage)
2. Drag-and-drop into your unity project (Aklternatively, Assets > Import Package > Custom Package...)
3. Find the prefab in `Assets/UltraDev/OWOPro/OWOPro.prefab`

![Prefab Location](images/Prefab.png)

4. Drag the prefab onto your 

![Drag Prefabe onto Avatar](images/AddPrefab.png)

5. Adjust the contacts as desired, you can drag around the Game Object and change the shape and size

Note: Changes to collision tags and parameter name are not applied

![Adjust Contact Position](images/AdjustContacts.png)

6. (Optional) Adjust the generated contacts, you can change which and how many velocity contacts are generated, and enable/disable the blade type contact.

![Adjust Generator](images/AdjustGenerator.png)

7. Upload your avatar!


## Usage
1. Download the latest release for your platform
2. Run the application
3. Ensure you have OSC enabled in VRChat

<img src="images/OSCMain.png" alt="drawing" width="200"/>
<img src="images/OSCOptions.png" alt="drawing" width="200"/>
<img src="images/OSCEnabled.png" alt="drawing" width="200"/>


# Owner menu / intro spec (2026-10-21)

The owner's reference specification for the startup intros, main menu,
Single Player, Options, dialogs and menu behaviour, kept verbatim. Every
item is tracked in TASKS.md as T-177..T-192 (top of the priority order).
The owner can send Windows Portal 2 screenshots for any screen.

## 1. INTRO SEQUENCES AND MAIN MENU

Recreate the original Portal 2 startup and menu experience as closely as possible. The following intro sequences are missing and must play in the correct order, with their associated music and audio: the Valve logo turning its head, the "Powered by Source" intro screen, and the initial Portal 2 loading screen. Preserve the correct timing, transitions, audio, and progression into the main menu.

The main menu is missing the Robot Enrichment section. The gap between the Portal 2 logo at the top and "PLAY SINGLE PLAYER" is currently slightly too large, approximately one line of text. Adjust the spacing and font sizing as necessary to match the original. The logo and menu options do not align to the left in either the original or the current implementation, but the current alignment is technically correct; nevertheless, align them with one another because the inconsistency is visually distracting.

The Portal 2 logo must remain visible when navigating to other menus, including Options. It must not disappear when leaving the main menu.

Menu item hover behavior must match the original. The current implementation changes the highlight background's color and dimensions, including shrinking it to fit the selected text. In the original, the highlight maintains a consistent size across all menu options instead of resizing to fit each label. Reproduce the original sizing, positioning, and highlighting behavior.

Implement the missing tile-flipping transitions. These include the fullscreen transition from the intro screen to the main menu; the tile animation when opening Options, Extras, Community Test Chambers, Play Single Player, and Play Cooperative Campaign; and the corresponding animations when navigating between subsections, such as Video and Advanced Video. Match the original animation's direction, timing, appearance, and behavior rather than substituting a generic fade or slide.

Restore the appropriate original navigation sounds, including menu-item hover sounds, selection/click sounds, and sounds associated with switching menus and sections. Ensure sounds occur at the correct times and are not repeatedly triggered by a single interaction.

Robot Enrichment and its icon, Play Cooperative Campaign, Extras, and Community Test Chambers must remain in their original positions but be greyed out, non-hoverable, and non-clickable. These unavailable features must not accidentally respond to mouse or keyboard navigation.

## 2. PLAY SINGLE PLAYER MENU

The Play Single Player menu must contain Continue Game, New Game, Load Game, Challenge Mode, and Developer Commentary. Challenge Mode and Developer Commentary must remain greyed out, non-hoverable, and non-clickable for now.

Continue Game must load the most recent valid save and resume the game from the correct location and state. The background video shown behind the menu must correspond to the chapter associated with the most recent save. Handle the absence of saves gracefully rather than loading an invalid or nonexistent save.

New Game must open the chapter-selection menu. Chapters the player has already played must have the appropriate hover and selection behavior. Each chapter entry must display the corresponding screenshot and chapter title beneath it, following the original save-menu layout. Chapter titles must use the bold styling observed in the original game. Selecting a chapter must start the appropriate chapter rather than loading an unrelated location.

Load Game must display the player's available saves, with the most recent saves presented in the appropriate order. Each save entry must include its screenshot, save title, and chapter name. Format save dates and times using the original style, such as "Saturday, Jun 27 7:32 PM". The displayed date and time must come from the actual save timestamp and must never be hardcoded. Display "Auto Save" in the top-right corner of the screenshot only when the save is an autosave. Manual saves must not display that label. The chapter name and save title must use the bold styling observed in the original game.

Save entries must load the correct saved game when selected. Ensure the displayed screenshot, timestamp, chapter name, and autosave status correspond to the same save. Handle empty save lists, invalid saves, missing screenshots, duplicate timestamps, and other unusual save states without crashing or displaying misleading information.

## 3. OPTIONS MENU — GENERAL BEHAVIOR

Recreate the original Options menu structure, appearance, navigation, and behavior. Settings must be functional rather than decorative: changing a control must update the corresponding setting, and the selected value must accurately reflect the active or pending configuration.

When any setting has been changed, including a setting inside Advanced Video, display Use Defaults at the bottom alongside Apply and Back, matching the original behavior. Use Defaults must restore the relevant settings to their defaults. Apply must apply pending changes, while Back must follow the correct behavior for saved, applied, or unapplied changes. Do not show Use Defaults when there are no changes, unless the original game does so in that context.

The bottom navigation buttons must resemble the original buttons, including the brighter highlight state when hovered or focused and the blue appearance when clicked. Keep button sizes, spacing, alignment, focus states, and transitions consistent with the original.

Settings must persist appropriately when navigating between sections, closing and reopening menus, and restarting the game. Distinguish pending changes from applied changes. Backing out of a submenu must not silently discard pending changes unless the original behavior requires it. Verify that Apply, Back, Use Defaults, and any confirmation dialogs behave consistently across the main Options menu and Advanced Video.

## 4. AUDIO SETTINGS

The Audio menu must include Sound Quality, Puzzle Maker Sounds, Closed Captioning, Closed Captioning Language, Enable Voice, Push to Talk Key, and Setup Microphone.

Sound Quality must provide Low and High options. Puzzle Maker Sounds must provide Disabled and Enabled options. Rename the existing "Captioning" setting to "Closed Captioning" and provide None, Subtitles Only, and Full Captions options. Add the Closed Captioning Language setting and reproduce the original option-selection behavior.

Enable Voice must provide Open Microphone, Push to Talk, and Disabled. Push to Talk Key must be greyed out and unavailable unless Push to Talk is selected. When Push to Talk is enabled, the key setting must become available.

The default Push to Talk Key must be C. Selecting the key-setting control must navigate to Edit Keys/Buttons, scroll to the Push to Talk Key binding, and highlight it so the player can change it immediately. The selected key must update the binding and display correctly when returning to the Audio menu.

Setup Microphone must always appear in bold, matching the original. Selecting it must open the Steam overlay, where supported. Do not replace this with an unrelated in-game screen or a nonfunctional button.

Verify that audio settings actually affect audio behavior. Closed Captioning options must produce the correct level of captions, language selection must affect caption language when supported, Puzzle Maker Sounds must enable or disable the relevant sounds, and voice settings must respect the selected mode. Settings that depend on Steam or unavailable voice infrastructure must fail gracefully without crashing.

## 5. VIDEO SETTINGS

The Video menu must contain Aspect Ratio, Resolution, Display Mode, Laptop Power Savings, Splitscreen Direction, and Advanced Video.

Aspect Ratio must provide Widescreen (16:9), Widescreen (16:10), and Normal (4:3). The default must correspond to the native display's appropriate aspect ratio.

Resolution must list resolutions supported by the active display and rendering configuration. The native resolution must be selected by default when appropriate and identified with a "(Native)" label. Do not hardcode a single display's resolution list or show unsupported resolutions as if they were available.

Display Mode must provide Windowed (No Border), Fullscreen, and Run in a Window. Each option must change the actual display behavior as intended, not merely change the displayed selection. Handle switching modes safely and preserve a usable interface if a display-mode change fails.

Laptop Power Savings must provide Enabled and Disabled. Splitscreen Direction must provide Default (Horizontal) and Vertical. These settings must display and apply the corresponding values where supported.

Advanced Video must always appear in bold, matching the original, and open its own settings section. Advanced Video must be scrollable to accommodate its additional options. Do not enlarge the entire menu window to fit all options at once.

Test resolution changes, aspect-ratio changes, display-mode switching, and returning to the previous menu. Ensure that the UI remains correctly scaled and usable after display changes and that selected values reflect the actual active configuration. Restore a safe display configuration if a change cannot be applied.

## 6. ADVANCED VIDEO SETTINGS

Advanced Video must contain the following settings in this exact order: Anti-Aliasing Mode, Filtering Mode, Wait for Vertical Sync, Multicore Rendering, Shader Detail, Effect Detail, Model / Texture Detail, and Paged Pool Memory Available. Additional project-specific graphics settings may follow these original settings, but must not disrupt their order or the original layout.

Anti-Aliasing Mode must provide None, 2x MSAA, and 4x MSAA. Filtering Mode must provide Bilinear, Trilinear, Anisotropic 2x, Anisotropic 4x, Anisotropic 8x, and Anisotropic 16x. Wait for Vertical Sync must provide Disabled, Enabled (Double Buffered), and Enabled (Triple Buffered). Multicore Rendering must provide Enabled and Disabled. Shader Detail must provide Low, Medium, High, and Very High. Effect Detail must provide Low, Medium, and High. Model / Texture Detail must provide Low, Medium, and High. Paged Pool Memory Available must provide Low, Medium, and High.

The section must be scrollable rather than enlarging the window. All options must remain accessible, with correct spacing, selection indicators, hover behavior, and keyboard navigation. Scrolling must not interfere with selecting a setting or activating the bottom navigation buttons.

Use Defaults must appear when settings have been changed, alongside Apply and Back. Use Defaults must reset the appropriate settings to their original defaults. Apply must apply the pending changes and automatically return to the previous menu. Back with unapplied changes must open a confirmation dialog displaying "Changes have been made. Discard current changes?" with Discard and Cancel.

Selecting Discard must discard the pending changes and leave Advanced Video, returning to the previous menu. Selecting Cancel must close the dialog and return to Advanced Video while preserving the pending changes. Applying changes must apply them and automatically return to the previous menu. If applying a graphics setting requires a restart, a display reset, or another confirmation, handle that appropriately without losing the player's ability to recover.

Test that the controls affect the rendering configuration wherever the underlying renderer supports them. If a specific original Source-engine option cannot be reproduced literally in the new engine, preserve the original menu structure and intended behavior as closely as possible, and map it to the closest meaningful supported setting rather than presenting a completely nonfunctional control. Keep any project-specific additions separate from the original settings.

## 7. CONTROLLER SETTINGS

For now, the Controller menu must contain only one setting: Controller, with Enabled and Disabled options. Match the original presentation and interaction behavior.

The setting must actually enable or disable controller input where supported. Its selected value must persist appropriately and accurately reflect the active configuration. Do not add unrelated controller settings to this menu at this stage.

## 8. KEYBOARD / MOUSE SETTINGS

The Keyboard / Mouse menu must contain Edit Keys/Buttons, Reverse Mouse, Mouse Sensitivity, Raw Mouse Input, Mouse Acceleration, Acceleration Amount, and Allow Developer Console.

Reverse Mouse must provide Enabled and Disabled. Mouse Sensitivity must use a functional slider. Raw Mouse Input must provide Enabled and Disabled. Mouse Acceleration must provide Enabled and Disabled. Acceleration Amount must use a functional slider. Allow Developer Console must provide Enabled and Disabled.

Edit Keys/Buttons must always appear in bold, matching the original. Selecting it must open an organized key-binding menu with Movement as a left-side section heading and Key/Button as a right-side column heading. Repeat this two-column structure for the relevant sections and bindings, keeping alignment, spacing, and navigation consistent with the original.

Movement bindings must include Move Forward, Move Back, Move Left (Strafe), Move Right (Strafe), Jump, and Duck.

Actions bindings must include Fire Blue Portal, Fire Orange Portal, Use Item (Buttons, Machines, ...), Toggle Zoom, Zoom In, and Zoom Out.

Co-op bindings must include Partner Remote View, Communication Menu, Gesture Menu, Push to Talk Key, and Chat Message.

Miscellaneous bindings must include View Leaderboard, Take Screenshot, Quick Save, Quick Load, Pause Game, Quit Game, Toggle Developer Console, and Swap Splitscreen Control.

Miscellaneous Movement bindings must include Turn Left, Turn Right, Look Up, Look Down, Strafe Modifier, and Keyboard Look Modifier.

The bottom buttons must include Back, Edit Key, Clear Key, and Use Defaults. Edit Key must allow the selected binding to be reassigned. Clear Key must clear the selected binding when appropriate. Use Defaults must restore the default bindings. Back must return to the previous menu without unintentionally discarding valid binding changes.

Key rebinding must support the intended keyboard or controller input, display the newly assigned binding, and update the actual in-game action. Prevent invalid or conflicting bindings from silently producing broken controls. Provide a clear way to cancel an in-progress binding operation. Verify that bindings remain correct after leaving and reopening the menu and after restarting the game.

When navigating to a binding from another menu, such as Push to Talk Key in Audio, the key-binding menu must scroll to the correct entry and highlight it. The user should not have to locate the binding manually.

Mouse Sensitivity and Acceleration Amount must change the corresponding input behavior. Raw Mouse Input and Reverse Mouse must also work as selected. Allow Developer Console must control whether the developer console can be opened.

## 9. SLIDER APPEARANCE AND BEHAVIOR

Recreate the original slider appearance. The original slider has a small arrow above the slider at its default position; add this indicator and ensure it appears at the correct position. Remove the arrows on the sides of the sliders because the original does not have them.

Apply these changes to Mouse Sensitivity, Acceleration Amount, and any shared slider component used elsewhere in the menus. Do not fix only one slider while leaving the others inconsistent.

Slider values must respond correctly to user input, remain within their valid ranges, display the correct position, and preserve their values according to the relevant settings behavior. Verify mouse interaction, keyboard navigation, focus behavior, and the visual position of the default indicator.

## 10. QUIT CONFIRMATION DIALOG

Clicking Quit on the main menu must open a confirmation dialog instead of immediately closing the game. Recreate the original dialog's appearance, animation, and button behavior.

The dialog must display the title "Portal 2", the message "Are you sure you want to quit?", and the buttons Quit and Cancel. Use the same general appearance and tile-flipping animation as the "Changes have been made. Discard current changes?" dialog, including the original button styling and transitions.

Selecting Quit must exit the application. Selecting Cancel must close the dialog and return to the main menu without quitting. Opening or closing the dialog must not accidentally trigger the underlying menu option. Ensure the dialog handles mouse input, keyboard focus, and confirmation/cancellation consistently.

## 11. SHARED MENU NAVIGATION AND INTERACTION

All menu sections must navigate correctly between one another, with the appropriate original transitions and sounds. Back must return to the correct parent section instead of always returning to the main menu. Navigation history must be preserved where the original game preserves it, including returning from Advanced Video to Video and returning from key bindings to Keyboard / Mouse.

Mouse hover, clicking, keyboard navigation, focus, and selection must all behave consistently. Greyed-out options must remain unavailable to both mouse and keyboard interaction. Moving between items must not cause unintended selections, repeated sounds, incorrectly sized highlights, or focus loss.

The Portal 2 logo and common menu elements must remain in their correct positions when navigating between sections. Menus must not jump, resize unexpectedly, or lose their layout when the number of available options changes. Preserve the original spacing, typography, button styling, alignment, highlight sizing, and tile-flipping transitions wherever applicable.

Use shared components for common controls such as buttons, sliders, menu highlights, and confirmation dialogs where appropriate, so fixes to shared behavior remain consistent throughout the menus.

## 12. SETTINGS STATE, CONFIRMATIONS, AND PERSISTENCE

Settings must have a clear distinction between their displayed values, pending changes, and applied values. Changing a control must mark the relevant settings as changed. Returning to a menu must not silently revert pending changes unless the original behavior calls for it.

Apply must apply the relevant changes. Use Defaults must restore the appropriate default values. Back must preserve or discard changes according to the relevant menu's original behavior. Where a confirmation is required, Cancel must preserve the current pending changes and Discard must discard them. Confirmation dialogs must not accidentally activate menu controls behind them.

Persist settings that should survive navigation and application restarts. When reopening a settings menu, show the values actually saved or currently active. Handle missing, invalid, or outdated settings data safely by restoring appropriate defaults instead of crashing.

Test settings individually and in combination, including changing multiple settings, using defaults, applying changes, cancelling a discard prompt, confirming a discard, leaving a section without changes, reopening menus, and restarting the application. Ensure one setting does not unintentionally reset another.

## 13. SAVE AND CHAPTER MENU EDGE CASES

Continue Game, New Game, and Load Game must all handle the player's progress correctly. Continue Game must use the most recent valid save. New Game must provide the appropriate chapter selection and start the correct chapter. Load Game must display the available saves accurately and load the selected save.

Verify the behavior when no saves exist, when only autosaves exist, when manual and autosaves coexist, when save screenshots are missing, when a save cannot be loaded, and when multiple saves have similar timestamps. The interface must never invent timestamps, chapter names, save titles, or autosave indicators.

Menu background videos must correspond to the relevant chapter or menu state where the original game uses them. If the required video or asset is unavailable, handle the missing asset gracefully rather than displaying a broken element or crashing.

## 14. VISUAL SCALING, DISPLAY CHANGES, AND ACCESSIBILITY OF CONTROLS

The menu must remain usable at supported resolutions, aspect ratios, and display modes. Preserve the intended original layout while scaling appropriately to the active display. Avoid clipped text, overlapping controls, incorrectly positioned highlights, and menus that extend beyond the screen.

Test native resolution, non-native resolutions, fullscreen, borderless windowed mode, windowed mode, and different aspect ratios. Check that returning from a display-mode change does not leave the UI at the wrong scale or position. Ensure scrollable sections remain usable at smaller resolutions.

All functional controls must be accessible through the intended input methods. Keyboard focus must be visible and must move in a sensible order. Mouse hover and keyboard focus should produce consistent visual states. Disabled options must be visibly disabled and must not respond to input. Dialogs must correctly capture interaction until dismissed.

## 15. INTRO, AUDIO, ANIMATION, AND REGRESSION TESTING

Verify the complete startup sequence from launch through the intro screens and into the main menu, including the order of the Valve logo, "Powered by Source" screen, Portal 2 loading screen, and main menu. Confirm that the appropriate audio and music play at the correct points and that transitions do not skip, overlap, or repeat unexpectedly.

Test every menu transition, hover sound, click sound, tile-flipping animation, dialog animation, and return-navigation path. Compare the implementation directly against the original Portal 2 rather than relying only on memory or assumptions. Check animation timing, highlight size, logo visibility, text spacing, button appearance, and menu layout.

Perform regression testing after changes to shared menu components. A fix to one section must not break sliders, buttons, dialogs, scrolling, navigation, settings persistence, or visual alignment in another section. Test both mouse and keyboard interaction after each significant UI change.

## 16. CURRENT SCOPE RESTRICTIONS AND FINAL GOAL

Robot Enrichment and its icon, Play Cooperative Campaign, Extras, Community Test Chambers, Developer Commentary, and Challenge Mode must remain in their correct original positions and be greyed out, non-hoverable, and non-clickable for now. Do not implement these unavailable modes as active features or allow them to open incomplete screens.

The goal is to reproduce the original Portal 2 menu structure, intro sequence, navigation, animations, audio, layout, settings, save and chapter menus, dialogs, and interaction behavior as faithfully as possible. Every behavior documented above is required. This is a reference specification for what the menus should do and how they should look, not a list of optional improvements.

Compare against the original game throughout implementation and testing. Where an original Source-engine feature cannot be reproduced exactly because of the new engine or platform, preserve the original presentation and intended behavior as closely as possible, using the closest functional equivalent without pretending unsupported behavior is implemented. Keep unavailable modes disabled as specified, and prioritize correctness, consistency, and faithful recreation over adding unrelated features.

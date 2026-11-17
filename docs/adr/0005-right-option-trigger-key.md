# Right Option as the default Trigger key, not Fn

The Trigger key defaults to Right Option (hold for Push-to-talk, double-tap for Hands-free, with Shift for a Quick note), rebindable in settings, and is read through a `CGEventTap`. Fn/🌐 was the first choice but macOS binds it to the emoji picker, input source switching or Apple Dictation by default, so using it would mean asking every user to change a system setting or fighting the OS for the key.

## Considered options

- **Fn/🌐 with onboarding that asks the user to set "Press 🌐 key to: Do Nothing":** nicest key, but an extra setup step that breaks if the setting changes.
- **Swallowing Fn with an active event tap:** no setup, but not reliable across macOS versions.
- **Right Command:** easy to reach, but many people use it in shortcuts.

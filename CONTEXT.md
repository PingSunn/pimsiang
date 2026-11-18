# PimSiang

PimSiang (พิมพ์เสียง) is a desktop app that turns spoken Thai into text, either typed into the app the user is working in or saved as a note.

## Language

### Speaking

**Utterance**:
One continuous span of speech captured between pressing and finishing the trigger.
_Avoid_: Recording, clip, session

**Trigger key**:
The key the user holds or double-taps to start an Utterance. Right Option by default, rebindable.
_Avoid_: Hotkey, shortcut

**Push-to-talk**:
Capturing an Utterance only while the Trigger key is held down.
_Avoid_: Hold mode

**Hands-free**:
Capturing an Utterance that starts on a double-tap of the Trigger key and ends on the next tap.
_Avoid_: Toggle mode, continuous mode

### Text

**Partial text**:
The provisional transcription of an Utterance in progress, which may still change.
_Avoid_: Interim result, live text

**Final text**:
The settled transcription of a finished Utterance.
_Avoid_: Result, output

**Dictation**:
Delivering an Utterance's Final text into the focused text field of another app.
_Avoid_: Typing, insert

**Note**:
An Utterance's Final text kept in PimSiang instead of being delivered to another app, either because the user asked for it or because no text field was focused.
_Avoid_: Memo, quick note (as a noun for the item)

**Quick note**:
The way of speaking that produces a Note on purpose (Trigger key with Shift).
_Avoid_: Note mode

**History**:
Every Final text PimSiang has produced, whether delivered by Dictation or kept as a Note. Text only, never audio.
_Avoid_: Log, archive

### Interface

**Pill**:
The small floating indicator on the active screen that shows listening state and Partial text.
_Avoid_: Hotbar, overlay, HUD, island

**Pill placement**:
Where the Pill appears: Notch (it comes out of the notch at the top centre, or sits below the menu bar when the screen has no notch) or Floating (it sits at the Pill position the user chose). Floating is the default.
_Avoid_: Pill mode, anchor

**Pill position**:
The spot the user chose for a Floating Pill, remembered as a share of the screen so it lands in the same relative place on any screen.
_Avoid_: Pill coordinates, offset

**Placing**:
The short mode in which a sample Pill can be dragged to choose the Pill position, ended with Done or Esc.
_Avoid_: Edit mode, drag mode

**Engine**:
The part that turns audio into Partial and Final text; swappable behind one interface.
_Avoid_: Backend, recognizer, ASR (as a component name)

**Model**:
A set of speech recognition weights the Engine can run. Exactly one Model is the Active model at a time; others may sit on disk unused.
_Avoid_: Weights, checkpoint

**Active model**:
The Model the Engine uses for the next Utterance, chosen by the user in settings.
_Avoid_: Current model, selected model

**Model catalog**:
The list of Models PimSiang knows how to download and verify, fixed per release.
_Avoid_: Model registry, model store

**Custom model**:
A Model the user points PimSiang at in a folder on their machine, outside the Model catalog and not verified by PimSiang.
_Avoid_: Local model, user model

**Language mode**:
How a Model writes English words spoken inside Thai speech (`th-mix` in Latin letters, `th-TH` in Thai script). Only some Models offer it.
_Avoid_: Locale, language setting

**Recognition mode**:
How a Model weighs the speech it hears: Standard for speech meant to be written down, Conversation for casual talk. Only some Models offer it.
_Avoid_: Mode (alone), speaking style

**Precision**:
Which build of a Model's weights the Engine runs: Accurate (full size, the default) or Light (smaller and faster, slightly less accurate).
_Avoid_: Quality, model size, int8/fp32 (as user-facing names)

# Partial text stays in the Pill; Final text is pasted once on release

During an Utterance the Partial text is shown only in the Pill. When the Utterance ends, the Final text is delivered in one paste (put on the pasteboard, send ⌘V, then restore the previous pasteboard), so the target app sees one clean insertion that undoes in one step. Before pasting, the Accessibility API is asked whether a text field is focused; if not, the text is kept as a Note and copied to the pasteboard instead of being lost. When the Accessibility answer is unclear (some Electron apps misreport), PimSiang pastes anyway.

## Considered options

- **Typing Partial text into the target app as it arrives:** feels faster, but every model correction means deleting and retyping, which breaks undo and confuses some apps.
- **Always pasting without the Accessibility check:** simpler, but text spoken with nothing focused would only survive in History.

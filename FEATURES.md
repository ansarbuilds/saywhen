# Features

Each row is something the app actually does. "Shipped" means it was already in the beta before this series. Later rows name the pull request that added them. Nothing here is a planned idea.

## Listening

1. Camera and microphone must agree before hands-free listening starts. Shipped.
2. 500 ms audio pre-roll so the first syllable is kept. Shipped.
3. Silero voice activity detection. Shipped.
4. Head-pose gate with separate enter and exit cones. Shipped.
5. Per-user gaze calibration. Shipped.
6. Side-camera mode for a webcam off to the side. Shipped.
7. Lip, smile, chew, and yawn separation. Shipped.
8. Adaptive camera frame rate while you look away. Shipped.
9. Low, full, and auto camera power. [PR 22](https://github.com/ansarbuilds/saywhen/pull/22). Low power is 480 scheduled frames a minute while facing the screen, against 909 at the normal tick. That is not a battery measurement.
10. Manual hotkey fallback. Shipped.
11. Recordable push-to-talk shortcut. Shipped.
12. Opt-in wake word when you are not facing the screen. Shipped.
13. Wake-word chime. Shipped.
14. Three-phrase voice enrollment. Shipped on main, not in the v0.2.1 installer.
15. Quiet hours that block hands-free start and leave the hotkey alone. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
16. Minimum speech length so coughs are dropped. Shipped.
17. Adaptive silence timeout. Shipped.
18. Post-roll so the last word is not clipped. Shipped.
19. Sleep and lock cancel an in-flight preview. Shipped.
20. Microphone hot-plug recovery. Shipped.

## Speech and words

21. On-device Whisper tiny, base, small, and Large v3 Turbo. Shipped. Large v3 Turbo is on main, not in v0.2.1.
22. On-device Parakeet TDT 0.6B v2. Shipped on main, not in v0.2.1.
23. Model download with progress. Shipped.
24. Model advice for speed, balanced, or accuracy from the models that are installed. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26). It is a preference order, not a measured ranking.
25. English-only models stay in English. [PR 22](https://github.com/ansarbuilds/saywhen/pull/22).
26. Language choice for Large v3 Turbo. [PR 22](https://github.com/ansarbuilds/saywhen/pull/22).
27. Local developer cleanup for CLI words and flags. Shipped.
28. Custom vocabulary. Shipped.
29. Snippets. [PR 22](https://github.com/ansarbuilds/saywhen/pull/22).
30. Spoken punctuation. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
31. Capitalize sentences. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
32. Scratch that. Shipped.
33. Paste only voice override. Shipped.
34. Send it / submit it end phrase. Shipped.
35. Hallucination filter. Shipped.
36. Opt-in local or bring-your-own-key clean-up. Shipped.
37. API key stored in the system keychain or a private file, never on a command line. [PR 21](https://github.com/ansarbuilds/saywhen/pull/21).

## Sending

38. Pause, nod, phrase, and type-only send styles. Shipped.
39. Cancelable preview before every automatic send. Shipped.
40. Cancel with Esc, a head shake, or by talking again. Shipped.
41. Nod and shake gestures. Shipped.
42. Enter only in trusted apps. Shipped.
43. Per-app send style, extra words, and language. [PR 22](https://github.com/ansarbuilds/saywhen/pull/22).
44. Abort if the front app changes during the preview. Shipped.
45. Skip Enter if focus moves after the paste. Shipped.
46. Destructive-command warning that does not press Enter. Shipped.
47. Multi-monitor gaze downgrade to type-only. Shipped.
48. Whole-utterance commands such as pause listening and low power mode. [PR 22](https://github.com/ansarbuilds/saywhen/pull/22).
49. Paste the last transcript again. Shipped.
50. Practice box that does not type into another app. Shipped.

## Window

51. Floating click-through status pill. Shipped.
52. Pill position: bottom center, bottom left, bottom right, or top. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
53. Move the pill to the display under the pointer. Shipped.
54. Tray menu, close-to-background, and Dock reopen. Shipped.
55. Light, dark, and system theme. Shipped.
56. Eight colour palettes and mood glow. Shipped.
57. High contrast, reduce motion, and four text sizes. Shipped.
58. Command palette. Shipped.
59. First-run intro and setup checklist. Shipped.
60. In-app Guide. Shipped.
61. Keyboard shortcut list. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
62. Onboarding focus trap. Shipped.

## History and settings

63. Seven-day local history with search. Shipped.
64. History search by app, `app:Name`. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
65. History export to Markdown. [PR 22](https://github.com/ansarbuilds/saywhen/pull/22).
66. One-click history purge. Shipped.
67. Local stats: utterances, words, and the app you dictate into most. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
68. Settings export and import, with API keys refused. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
69. False-trigger flag on a history card. Shipped.
70. Update channel remembered as stable or beta. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26). Nothing is downloaded because of it.

## Privacy and license

71. Camera frames stay in memory. Shipped.
72. Audio discarded after transcription unless you opt in. Shipped.
73. Session metrics that never store the transcript. Shipped.
74. Opt-in diagnostic notes that stay on this computer. [PR 26](https://github.com/ansarbuilds/saywhen/pull/26).
75. Fourteen-day local trial. [PR 24](https://github.com/ansarbuilds/saywhen/pull/24).
76. License enforcement off, soft, or required. [PR 24](https://github.com/ansarbuilds/saywhen/pull/24). Enforcement is off, so nothing is locked.
77. Development license keys, documented as public and not for sale. [PR 24](https://github.com/ansarbuilds/saywhen/pull/24).
78. Optional license server. [PR 24](https://github.com/ansarbuilds/saywhen/pull/24). No payment-provider secret is in the app.

## Safety of typing

79. Self-typing blocked. Shipped.
80. Exact app-name and bundle-id matching. Shipped.
81. macOS Secure Input and Windows elevation called out instead of failing quietly. Shipped.
82. Clipboard text and images restored when they are still the transcript. Shipped.
83. Held modifiers released before paste. Shipped.

84. Word count on each history card. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
85. Relative time, such as "5m ago", on each history card. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
86. History grouped into Today, Yesterday, and Earlier. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
87. Search highlights the matching words in a history card. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
88. A risky-command badge on a history card. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
89. License screen headline for trial, free, and Pro. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
90. Onboarding progress, such as "2 of 4 ready". [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
91. Tray menu wording comes from one tested list. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
92. HUD state word comes from the same hint map as the main window. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
93. Quiet-hours sentence under the time fields. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
94. A ten-icon set used by the design system. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
95. Shared spacing, type, motion, and focus tokens. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
96. Comfortable and compact density, remembered on this computer. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
97. History day headings that appear only when the day changes. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).
98. Menu bar preview in Settings that uses the same words as the tray. [PR 27](https://github.com/ansarbuilds/saywhen/pull/27).

The count in this file is 98. Each one is implemented. A later change should add a row only when the behavior lands in the code.

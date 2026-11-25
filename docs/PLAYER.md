# Player guide

## Open a file

Download the DMG for Apple Silicon on macOS 14 or later. Open it, drag **Hen Local Player** to **Applications**, then choose **Open File** (⌘O). To try the bundled Japanese clip, choose **••• → Play Sample** in the opening window. [Download the latest release](https://github.com/Hen-Local/Hen-Local-Player/releases/latest)

- **Open Folder** lists playable files together. Opening a single file also puts other media from its folder in **Playlist**. Use **Add Files** to make your own queue.
- **Open URL** (⌘L) previews a video link before downloading it. The player offers to install the URL tools on first use.
- Choose a recent file in the opening window to resume it. **⌘W** closes the window. Click the Dock icon to open the player again, or press **⌘Q** to quit.

## Generate bilingual subtitles

1. Open **Settings → General → Model Setup**. On Apple Silicon the recommended components include Qwen3-ASR 1.7B for speech recognition and Hy-MT2 1.8B for translation. Choose **Download Missing Components**. Once installed, the models work offline. You can play local files and use existing subtitles before downloading any models.
2. Open a video and choose **Subtitles** in the playback controls. Under **Languages**, select the spoken and translation languages. Choose **Transcribe** to start. Progress appears at the bottom of the sidebar.
3. In **Display Language**, choose the original, the translation or both. Expand **Subtitle List** and click a line to jump to that point in the video. For audio-only files, the transcript scrolls with playback.

The player remembers generated subtitles for each file and audio track. **Regenerate** processes the track again. **Delete** clears its saved transcript and translations. To keep a separate SRT, VTT or ASS file, choose **Subtitles → Export Generated Subtitles…** (⌘E).

### Download sources

**Model Download Source** defaults to **Automatic**. It uses the system region and connection speed to choose a source. Choose **Mainland China** to prefer ModelScope and domestic mirrors, or **International** to prefer Hugging Face. Expand **Manual Download Links** to see the files.

### Existing subtitles

The **Subtitle Track** menu lists embedded text tracks and nearby SRT or WebVTT files. Choose **Import File…** to select another local subtitle file. To search online, choose **Find Online…** (⇧⌘D), enter the movie or episode title, select a language, then choose **Use Subtitle**. Guest search has a daily download allowance.

### Appearance and timing

**Appearance & Timing** controls text size, position, background opacity and subtitle delay. The defaults are 100% text size, 5% above the bottom of the picture and 70% background opacity. The reset arrow beside each slider restores its default. **Languages → Language Order** sets which line appears on top.

For context across lines, **Settings → Processing → Use Previous Dialogue for Translation** lets the model read preceding dialogue. This is enabled by default.

## Playback controls

Move the pointer over the picture to show the controls. Use the timeline or the ten-second skip buttons to seek. The speed control has presets from 0.5× to 4× and a slider from 0.25× to 5×. **Picture in Picture** opens a floating window for supported videos. Double-click the picture or press **F** for fullscreen and **Esc** to return.

The **Playlist** and **Subtitles** buttons open the left and right sidebars. Click the button again or press **Esc** to close the focused sidebar. In a compact window, you can also click the picture to dismiss the sidebar.

## Update the app

Choose **Settings → General → Software Update → Check for Updates**. Select a newer version, download it, then choose **Restart and Install**. You can enable **Download Updates Automatically** and choose when to restart.

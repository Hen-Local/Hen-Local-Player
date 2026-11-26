# Player guide

## Open a file

Download the DMG for Apple Silicon on macOS 14 or later. Open it, drag **Hen Local Player** to **Applications**, then choose **Open File / Open Folder**. On macOS you can select a media file or a folder in the same window. Open **Settings → About** to visit the project’s GitHub page. [Download the latest release](https://github.com/Hen-Local/Hen-Local-Player/releases/latest)

- Opening a folder puts its playable files in **Playlist**. Opening a single file also lists other media from its folder. Use **Add Files** to make your own queue.
- **Open URL** (⌘L) previews a video link before downloading it. The player offers to install the URL tools on first use.
- The opening window shows the four most recent items with their durations. When there are more, choose **Show More** to browse the full history. Select its name to resume playback. On macOS, hover over a row or focus it with the keyboard and choose **Open in Finder** to locate the file. **⌘W** closes the window. Click the Dock icon to open the player again, or press **⌘Q** to quit.

Choose **Settings → General → Interface Language** to switch among the 30 supported interface languages or follow your system preference. All interface languages are available offline.

## Generate bilingual subtitles

1. Open **Settings → General → Model Setup**. On Apple Silicon the recommended components include Qwen3-ASR 1.7B for speech recognition and Hy-MT2 1.8B for translation. Choose **Download Missing Components**. Once installed, the models work offline. You can play local files and use existing subtitles before downloading any models.
2. Open a video and choose **Subtitles** on the right of the playback bar. For a new user, **Translate To** uses the first supported language in the system’s preferred languages. Spoken input starts in English for other output languages. For English output, it uses another preferred spoken language if available, or detects the audio language automatically. You can change either choice under **Languages**. Choose **Generate Subtitles** to start.
3. **Display Language** initially shows the translation. Choose the original or both languages if desired. Expand **Subtitle List** and scroll down through the sidebar to read the captions. Click a line to jump to it. For audio-only files, you can also scroll the lyrics in the main view and choose **Now Playing** to return to the current line.

The player remembers generated subtitles for each file and audio track. **Regenerate** processes the track again. **Delete** clears its saved transcript and translations. To save a separate subtitle file, expand **Subtitle List** and choose **Export Subtitles…** (⌘E). The default format is SRT. You can also choose VTT or ASS.

### Download sources

**Model Download Source** defaults to **Automatic**. It uses the system region and connection speed to choose a source. Choose **Mainland China** to prefer ModelScope and domestic mirrors, or **International** to prefer Hugging Face. Expand **Manual Download Links** to see the files.

### Existing subtitles

The **Subtitle Track** menu lists embedded text tracks and nearby SRT or WebVTT files. Choose **Import File…** to select another local subtitle file. To search online, choose **Find Online…** (⇧⌘D), enter the movie or episode title, select a language, then choose **Use Subtitle**. Guest search has a daily download allowance.

### Appearance and timing

**Appearance & Timing** controls text size, position, background opacity and subtitle delay. The defaults are 100% text size, 5% above the bottom of the picture and 70% background opacity. The reset arrow beside each slider restores its default. **Languages → Language Order** sets which line appears on top.

For context across lines, **Settings → Processing → Use Previous Dialogue for Translation** lets the model read preceding dialogue. This is enabled by default.

## Playback controls

In windowed playback, the picture sets the window’s proportions. Drag any edge or corner to resize it. Switching between landscape, square and portrait videos adjusts the window to the next picture. Returning to the opening screen or playing an audio file restores free resizing. In fullscreen, space around the picture depends on the shape of the display.

Move the pointer over the picture to show the controls. Drag the volume slider to adjust the sound, or click the speaker to mute. Hover over the current speed to show presets from 0.5× to 4× and a slider from 0.25× to 5×. The buttons on either side of Play move to the previous or next file in the playlist. Drag the timeline to seek, or use the left and right arrow keys to jump five seconds. Hover over or click **…** at the right end of the bar for available playback outputs. **Picture in Picture** opens a floating window for supported videos. On macOS, **AirPlay** opens the system picker when a compatible receiver is available. Double-click the picture or press **F** for fullscreen and **Esc** to return.

On the timeline, white marks the elapsed time. On macOS, the accent defaults to **Follow System**. Changing the accent in **System Settings → Appearance** updates the open player immediately. To choose a fixed color, open **Settings → General → Accent Color**. The current playlist item’s icon and the check marks for installed models use this color. Pink, orange and green use their neon shades on these indicators. Blue uses a lighter shade against dark glass.

Ahead of the playhead, light gray shows media already buffered for playback and dark gray shows intervals still to be read. While subtitles are being prepared, hover over or focus the timeline to see their progress on a separate thin strip underneath.

The **Playlist** and **Subtitles** buttons on the right of the playback bar open panels over the video and controls. Click the **×** in a panel or press **Esc** to close the panel you used last. Click the exposed picture to dismiss all open panels, or drag it to move the window with the panels open. Both gestures preserve the current playback state.

In **Playlist**, a filmstrip marks a video and a waveform marks an audio file. **Sort** lets you restore the original order, sort by name in either direction, or sort by the file’s modification date. Sorting keeps the current file playing. Previous and Next follow the order shown.

## Update the app

Open **Settings → General** to see the installed version. Choose **Check for Updates** to check the project’s GitHub releases. Select a newer version, download it, then choose **Restart and Install**. You can enable **Download Updates Automatically** and choose when to restart.

#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
#import <objc/runtime.h>
#include <stdbool.h>

// Resize the live window on the main run loop so WebKit keeps drawing the movie.
@interface HLPFullscreenAnimation : NSAnimation
@property(nonatomic, weak) NSWindow *window;
@property(nonatomic) NSRect fromFrame;
@property(nonatomic) NSRect toFrame;
@end

@implementation HLPFullscreenAnimation
- (void)setCurrentProgress:(NSAnimationProgress)progress {
    [super setCurrentProgress:progress];
    if (!self.window) return;
    NSRect from = self.fromFrame, to = self.toFrame;
    CGFloat t = self.currentValue;
    NSRect frame = NSMakeRect(from.origin.x + (to.origin.x - from.origin.x) * t,
                              from.origin.y + (to.origin.y - from.origin.y) * t,
                              from.size.width + (to.size.width - from.size.width) * t,
                              from.size.height + (to.size.height - from.size.height) * t);
    [self.window setFrame:progress >= 1 ? to : NSIntegralRect(frame) display:YES];
}
@end

// AppKit can supply two fullscreen menu items, including a keyboard-layout
// dependent shortcut. Keep both system items and their existing shortcuts.
@interface HLPPlayerFullscreen : NSObject
@property(nonatomic, weak) WKWebView *webview;
@property(nonatomic, strong) NSMapTable<NSMenuItem *, id> *previousTargets;
@property(nonatomic, strong) HLPFullscreenAnimation *animation;
@property(nonatomic) BOOL enabled;
@property(nonatomic) NSRect windowedFrame;
@property(nonatomic) NSWindowStyleMask windowedStyle;
@property(nonatomic) NSApplicationPresentationOptions windowedPresentation;
@property(nonatomic) BOOL windowedMovable;
- (void)setFullscreen:(BOOL)enabled;
@end

@implementation HLPPlayerFullscreen
- (void)toggleFullScreen:(id)sender {
    [self.webview evaluateJavaScript:@"window.dispatchEvent(new Event('hlp:exit-player-fullscreen'))"
                  completionHandler:nil];
}
- (void)performClose:(id)sender {
    // AppKit's normal close target has no title-bar close button in this mode.
    [self.webview evaluateJavaScript:@"window.dispatchEvent(new Event('hlp:close-player-window'))"
                  completionHandler:nil];
}
- (void)routeMenu {
    self.previousTargets = [NSMapTable weakToWeakObjectsMapTable];
    for (NSMenuItem *group in NSApp.mainMenu.itemArray) {
        for (NSMenuItem *item in group.submenu.itemArray) {
            if (item.action != @selector(toggleFullScreen:) && item.action != @selector(performClose:)) continue;
            [self.previousTargets setObject:item.target ?: NSNull.null forKey:item];
            item.target = self;
        }
    }
}
- (void)restore {
    for (NSMenuItem *item in self.previousTargets.keyEnumerator) {
        if (item.target == self) {
            id previous = [self.previousTargets objectForKey:item];
            item.target = previous == NSNull.null ? nil : previous;
        }
    }
    [self.previousTargets removeAllObjects];
}
- (void)setFullscreen:(BOOL)enabled {
    NSWindow *window = self.webview.window;
    if (!window.screen || self.enabled == enabled || (window.styleMask & NSWindowStyleMaskFullScreen)) return;
    NSRect from = window.frame;
    BOOL reversing = self.animation.isAnimating;
    [self.animation stopAnimation];
    NSRect to;
    if (enabled) {
        // A quick re-entry must retain the intended windowed frame, not the
        // partially restored geometry at the moment of the second key press.
        self.windowedFrame = reversing ? self.animation.toFrame : from;
        self.windowedStyle = window.styleMask;
        self.windowedPresentation = NSApp.presentationOptions;
        self.windowedMovable = window.movable;
        to = window.screen.frame;
        NSApp.presentationOptions = NSApplicationPresentationAutoHideDock | NSApplicationPresentationAutoHideMenuBar;
        window.styleMask &= ~(NSWindowStyleMaskTitled | NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable);
        window.movable = NO;
        [self routeMenu];
    } else {
        to = self.windowedFrame;
        NSApp.presentationOptions = self.windowedPresentation;
        window.styleMask = self.windowedStyle;
        window.movable = self.windowedMovable;
        [self restore];
    }
    self.enabled = enabled;
    [window makeFirstResponder:self.webview];
    if (NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion || NSEqualRects(from, to)) {
        self.animation = nil;
        [window setFrame:to display:YES];
        return;
    }
    HLPFullscreenAnimation *animation =
        [[HLPFullscreenAnimation alloc] initWithDuration:0.22 animationCurve:NSAnimationEaseInOut];
    animation.window = window;
    animation.fromFrame = from;
    animation.toFrame = to;
    animation.frameRate = window.screen.maximumFramesPerSecond;
    animation.animationBlockingMode = NSAnimationNonblocking;
    self.animation = animation;
    [animation startAnimation];
}
- (void)dealloc {
    [_animation stopAnimation];
    if (_enabled) {
        NSApp.presentationOptions = _windowedPresentation;
        [self restore];
    }
}
@end

void hlp_set_player_fullscreen(WKWebView *webview, bool enabled) {
    static char key;
    HLPPlayerFullscreen *controller = objc_getAssociatedObject(webview, &key);
    if (!controller) {
        controller = [[HLPPlayerFullscreen alloc] init];
        controller.webview = webview;
        objc_setAssociatedObject(webview, &key, controller, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    }
    [controller setFullscreen:enabled];
}

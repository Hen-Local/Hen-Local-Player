#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
#import <objc/runtime.h>
#include <stdbool.h>

// AppKit can supply two fullscreen menu items, including a keyboard-layout
// dependent shortcut. Keep both system items and their existing shortcuts.
@interface HLPPlayerFullscreenTarget : NSObject
@property(nonatomic, weak) WKWebView *webview;
@property(nonatomic, strong) NSMapTable<NSMenuItem *, id> *previousTargets;
- (void)restore;
@end

@implementation HLPPlayerFullscreenTarget
- (void)toggleFullScreen:(id)sender {
    [self.webview evaluateJavaScript:@"window.dispatchEvent(new Event('hlp:exit-player-fullscreen'))"
                  completionHandler:nil];
}
- (void)performClose:(id)sender {
    // AppKit's normal close target has no title-bar close button in this mode.
    [self.webview evaluateJavaScript:@"window.dispatchEvent(new Event('hlp:close-player-window'))"
                  completionHandler:nil];
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
- (void)dealloc {
    [self restore];
}
@end

void hlp_set_player_fullscreen_target(WKWebView *webview, bool enabled) {
    static char targetKey;
    HLPPlayerFullscreenTarget *existing = objc_getAssociatedObject(webview, &targetKey);
    if (!enabled) {
        [existing restore];
        objc_setAssociatedObject(webview, &targetKey, nil, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
        return;
    }
    if (existing) return;
    HLPPlayerFullscreenTarget *target = [[HLPPlayerFullscreenTarget alloc] init];
    target.webview = webview;
    target.previousTargets = [NSMapTable weakToWeakObjectsMapTable];
    for (NSMenuItem *group in NSApp.mainMenu.itemArray) {
        for (NSMenuItem *item in group.submenu.itemArray) {
            if (item.action != @selector(toggleFullScreen:) && item.action != @selector(performClose:)) continue;
            [target.previousTargets setObject:item.target ?: NSNull.null forKey:item];
            item.target = target;
        }
    }
    objc_setAssociatedObject(webview, &targetKey, target, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
}

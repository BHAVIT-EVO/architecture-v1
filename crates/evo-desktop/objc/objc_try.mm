// ObjC exception guard for the WebKit surface calls.
//
// WebKit legitimately throws ObjC exceptions (e.g. invalid arguments).
// Rust cannot let a foreign exception unwind through its frames — it
// aborts the process. This shim runs the call inside @try/@catch and
// reports the exception's name and reason instead, so a WebKit refusal
// becomes an honest Evo error, never a crash.

#import <Foundation/Foundation.h>
#import <stdio.h>

extern "C" int evo_objc_try(void (*fn)(void *), void *ctx, char *out, unsigned long out_len) {
    @try {
        fn(ctx);
        return 1;
    } @catch (NSException *e) {
        if (out && out_len > 0) {
            snprintf(out, out_len, "%s: %s",
                     e.name.UTF8String ? e.name.UTF8String : "NSException",
                     e.reason.UTF8String ? e.reason.UTF8String : "");
        }
        return 0;
    } @catch (...) {
        // WebKit is C++ underneath: some failures surface as non-NS
        // exceptions. They are still refusals to report, not crashes.
        if (out && out_len > 0) {
            snprintf(out, out_len, "non-NSException (WebKit C++) exception");
        }
        return 0;
    }
}

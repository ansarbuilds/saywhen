#import <Cocoa/Cocoa.h>
#import <CoreText/CoreText.h>
#import <CoreGraphics/CoreGraphics.h>
#import <Accelerate/Accelerate.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const CGFloat W = 1080.0;
static const CGFloat H = 1920.0;
static const CGFloat CX = 540.0;
static const CGFloat CY = 960.0;
static const CGFloat FPS = 30.0;
static const CGFloat DT = 1.0 / 30.0;

#define MAX_AUDIO_FRAMES 1024
static float gAudioRms[MAX_AUDIO_FRAMES];
static float gAudioPeak[MAX_AUDIO_FRAMES];
static int gAudioFrameCount = 0;

// ============================================================================
// MATH & EASING PRIMITIVES
// ============================================================================
static inline double clampd(double x, double a, double b) {
    return x < a ? a : (x > b ? b : x);
}

static inline double clamp01(double x) {
    return clampd(x, 0.0, 1.0);
}

static inline double lerpd(double a, double b, double u) {
    return a + (b - a) * u;
}

static inline double seg(double t, double a, double b) {
    if (b <= a) return t >= b ? 1.0 : 0.0;
    return clamp01((t - a) / (b - a));
}

static inline double eo(double u) {
    double v = 1.0 - u;
    return 1.0 - v * v * v;
}

static inline double ei(double u) {
    return u * u * u;
}

static inline double eio(double u) {
    return u < 0.5 ? 4.0 * u * u * u : 1.0 - pow(-2.0 * u + 2.0, 3.0) / 2.0;
}

static inline double bo(double u) {
    const double c1 = 1.70158;
    const double c3 = c1 + 1.0;
    double d = u - 1.0;
    return 1.0 + c3 * d * d * d + c1 * d * d;
}

static inline double ease(double t, double a, double b) {
    return eio(seg(t, a, b));
}

static inline double easeOut(double t, double a, double b) {
    return eo(seg(t, a, b));
}

static inline uint32_t lcgNext(uint32_t *state) {
    *state = (*state) * 1664525u + 1013904223u;
    return *state;
}

static inline double lcgFloat(uint32_t *state) {
    return (double)lcgNext(state) / 4294967296.0;
}

// Convert top-down screen coordinate (x, y_from_top) to Cocoa bottom-up NSPoint
static inline NSPoint P(CGFloat x, CGFloat y) {
    return NSMakePoint(x, H - y);
}

static inline NSRect TD(CGFloat x, CGFloat y, CGFloat w, CGFloat h) {
    return NSMakeRect(x, H - y - h, w, h);
}

// ============================================================================
// BRAND COLOR PALETTE (saywhen src/style.css + film/src/SaywhenDemo.tsx)
// ============================================================================
static inline NSColor *rgba(double r, double g, double b, double a) {
    return [NSColor colorWithSRGBRed:clamp01(r / 255.0)
                               green:clamp01(g / 255.0)
                                blue:clamp01(b / 255.0)
                               alpha:clamp01(a)];
}

#define C_PLUM       rgba(34, 6, 28, 1.0)
#define C_PLUM_DEEP  rgba(51, 10, 43, 1.0)
#define C_PLUM_INK   rgba(42, 8, 35, 1.0)
#define C_AMBER      rgba(251, 179, 3, 1.0)
#define C_AMBER_DARK rgba(212, 134, 0, 1.0)
#define C_AMBER_SOFT rgba(255, 201, 77, 1.0)
#define C_CYAN       rgba(130, 232, 255, 1.0)
#define C_EMERALD    rgba(61, 220, 151, 1.0)
#define C_EMERALD_DK rgba(15, 138, 86, 1.0)
#define C_SILVER     rgba(223, 229, 236, 1.0)
#define C_ARMED      rgba(184, 204, 228, 1.0)
#define C_IVORY      rgba(246, 238, 244, 1.0)
#define C_WHITE      rgba(255, 255, 255, 1.0)
#define C_MUTED_DK   rgba(199, 184, 196, 1.0)
#define C_MUTED_LT   rgba(109, 86, 105, 1.0)
#define C_DANGER     rgba(255, 107, 94, 1.0)
#define C_DANGER_DK  rgba(220, 38, 38, 1.0)

// ============================================================================
// WAV AUDIO ENVELOPE READER (16-bit PCM Stereo)
// ============================================================================
static void loadWavEnvelope(const char *path, int totalFrames) {
    memset(gAudioRms, 0, sizeof(gAudioRms));
    memset(gAudioPeak, 0, sizeof(gAudioPeak));
    gAudioFrameCount = totalFrames;

    FILE *fp = fopen(path, "rb");
    if (!fp) {
        fprintf(stderr, "Warning: could not open WAV %s for reactive envelope\n", path);
        return;
    }

    uint8_t riff[12];
    if (fread(riff, 1, 12, fp) != 12 || memcmp(riff, "RIFF", 4) != 0) {
        fclose(fp);
        return;
    }

    uint32_t sampleRate = 44100;
    uint16_t channels = 2;
    uint32_t dataSize = 0;
    long dataOffset = 0;

    uint8_t chunkHdr[8];
    while (fread(chunkHdr, 1, 8, fp) == 8) {
        uint32_t chunkSize = (uint32_t)chunkHdr[4] |
                             ((uint32_t)chunkHdr[5] << 8) |
                             ((uint32_t)chunkHdr[6] << 16) |
                             ((uint32_t)chunkHdr[7] << 24);
        if (memcmp(chunkHdr, "fmt ", 4) == 0 && chunkSize >= 16) {
            uint8_t fmt[16];
            if (fread(fmt, 1, 16, fp) == 16) {
                channels = (uint16_t)fmt[2] | ((uint16_t)fmt[3] << 8);
                sampleRate = (uint32_t)fmt[4] |
                             ((uint32_t)fmt[5] << 8) |
                             ((uint32_t)fmt[6] << 16) |
                             ((uint32_t)fmt[7] << 24);
                if (chunkSize > 16) fseek(fp, (long)(chunkSize - 16), SEEK_CUR);
            }
        } else if (memcmp(chunkHdr, "data", 4) == 0) {
            dataOffset = ftell(fp);
            dataSize = chunkSize;
            break;
        } else {
            fseek(fp, (long)(chunkSize + (chunkSize & 1)), SEEK_CUR);
        }
    }

    if (dataSize == 0 || dataOffset == 0) {
        fclose(fp);
        return;
    }

    int16_t *pcm = (int16_t *)malloc(dataSize);
    if (!pcm) {
        fclose(fp);
        return;
    }
    fseek(fp, dataOffset, SEEK_SET);
    size_t bytesRead = fread(pcm, 1, dataSize, fp);
    fclose(fp);

    size_t totalStereoFrames = bytesRead / (sizeof(int16_t) * channels);
    size_t spf = (size_t)(sampleRate / 30);
    if (spf == 0) spf = 1470;

    float maxRms = 0.001f;
    for (int f = 0; f < totalFrames && f < MAX_AUDIO_FRAMES; f++) {
        size_t s0 = (size_t)f * spf;
        size_t s1 = s0 + spf;
        if (s0 >= totalStereoFrames) break;
        if (s1 > totalStereoFrames) s1 = totalStereoFrames;

        double acc = 0.0;
        double pk = 0.0;
        size_t count = s1 - s0;
        for (size_t s = s0; s < s1; s++) {
            double lv = pcm[s * channels] / 32768.0;
            double rv = (channels > 1) ? (pcm[s * channels + 1] / 32768.0) : lv;
            double m = 0.5 * (fabs(lv) + fabs(rv));
            acc += 0.5 * (lv * lv + rv * rv);
            if (m > pk) pk = m;
        }
        float r = (float)sqrt(acc / (double)(count > 0 ? count : 1));
        gAudioRms[f] = r;
        gAudioPeak[f] = (float)pk;
        if (r > maxRms) maxRms = r;
    }

    for (int f = 0; f < totalFrames && f < MAX_AUDIO_FRAMES; f++) {
        gAudioRms[f] = (float)clamp01(gAudioRms[f] / (maxRms * 0.78));
    }

    free(pcm);
}

static inline double frameRms(int f) {
    if (f < 0) f = 0;
    if (f >= MAX_AUDIO_FRAMES) f = MAX_AUDIO_FRAMES - 1;
    return (double)gAudioRms[f];
}

static inline double framePeak(int f) {
    if (f < 0) f = 0;
    if (f >= MAX_AUDIO_FRAMES) f = MAX_AUDIO_FRAMES - 1;
    return (double)gAudioPeak[f];
}

// ============================================================================
// FONT REGISTRATION & TYPOGRAPHY HELPERS
// ============================================================================
static NSMutableDictionary *gFontCache = nil;

static void registerBundledFonts(void) {
    gFontCache = [NSMutableDictionary dictionary];
    NSArray *paths = @[
        @"node_modules/@fontsource/instrument-serif/files/instrument-serif-latin-400-italic.woff2",
        @"node_modules/@fontsource/instrument-serif/files/instrument-serif-latin-400-normal.woff2",
        @"node_modules/@fontsource-variable/plus-jakarta-sans/files/plus-jakarta-sans-latin-wght-normal.woff2",
        @"node_modules/@fontsource-variable/plus-jakarta-sans/files/plus-jakarta-sans-latin-wght-italic.woff2",
        @"node_modules/@fontsource-variable/jetbrains-mono/files/jetbrains-mono-latin-wght-normal.woff2",
        @"node_modules/@fontsource-variable/jetbrains-mono/files/jetbrains-mono-latin-wght-italic.woff2"
    ];
    for (NSString *p in paths) {
        NSURL *u = [NSURL fileURLWithPath:p];
        CTFontManagerRegisterFontsForURL((__bridge CFURLRef)u, kCTFontManagerScopeProcess, NULL);
    }
}

static NSFont *fontSerif(CGFloat size, BOOL italic) {
    NSString *key = [NSString stringWithFormat:@"serif-%d-%.1f", italic, size];
    if (gFontCache[key]) return gFontCache[key];
    NSFont *f = [NSFont fontWithName:(italic ? @"InstrumentSerif-Italic" : @"InstrumentSerif-Regular") size:size];
    if (!f) f = [NSFont fontWithName:@"Georgia" size:size];
    gFontCache[key] = f;
    return f;
}

static NSFont *fontSans(CGFloat size, double weight) {
    NSString *key = [NSString stringWithFormat:@"sans-%.0f-%.1f", weight, size];
    if (gFontCache[key]) return gFontCache[key];
    FourCharCode wghtTag = 'wght';
    NSFontDescriptor *desc = [NSFontDescriptor fontDescriptorWithFontAttributes:@{
        NSFontFamilyAttribute: @"Plus Jakarta Sans",
        (__bridge NSString *)kCTFontVariationAttribute: @{ @(wghtTag): @(weight) }
    }];
    NSFont *f = [NSFont fontWithDescriptor:desc size:size];
    if (!f) f = [NSFont systemFontOfSize:size weight:weight >= 700 ? NSFontWeightBold : NSFontWeightMedium];
    gFontCache[key] = f;
    return f;
}

static NSFont *fontMono(CGFloat size, double weight) {
    NSString *key = [NSString stringWithFormat:@"mono-%.0f-%.1f", weight, size];
    if (gFontCache[key]) return gFontCache[key];
    FourCharCode wghtTag = 'wght';
    NSFontDescriptor *desc = [NSFontDescriptor fontDescriptorWithFontAttributes:@{
        NSFontFamilyAttribute: @"JetBrains Mono",
        (__bridge NSString *)kCTFontVariationAttribute: @{ @(wghtTag): @(weight) }
    }];
    NSFont *f = [NSFont fontWithDescriptor:desc size:size];
    if (!f) f = [NSFont monospacedSystemFontOfSize:size weight:weight >= 700 ? NSFontWeightBold : NSFontWeightRegular];
    gFontCache[key] = f;
    return f;
}

static CGSize measureText(NSString *str, NSFont *font, CGFloat tracking) {
    if (!str || str.length == 0) return CGSizeZero;
    NSDictionary *attrs = @{
        NSFontAttributeName: font,
        NSKernAttributeName: @(tracking)
    };
    return [str sizeWithAttributes:attrs];
}

static void drawText(NSString *str, CGFloat x, CGFloat y, NSFont *font, NSColor *color, CGFloat tracking) {
    if (!str || str.length == 0) return;
    NSDictionary *attrs = @{
        NSFontAttributeName: font,
        NSForegroundColorAttributeName: color,
        NSKernAttributeName: @(tracking)
    };
    CGSize sz = [str sizeWithAttributes:attrs];
    [str drawAtPoint:NSMakePoint(x, H - y - sz.height) withAttributes:attrs];
}

static void drawCenteredText(NSString *str, CGFloat y, NSFont *font, NSColor *color, CGFloat tracking) {
    CGSize sz = measureText(str, font, tracking);
    drawText(str, (W - sz.width) * 0.5, y, font, color, tracking);
}

static void drawRoundedBox(CGFloat x, CGFloat y, CGFloat w, CGFloat h, CGFloat r, NSColor *fill, NSColor *stroke, CGFloat strokeW) {
    NSBezierPath *path = [NSBezierPath bezierPathWithRoundedRect:TD(x, y, w, h) xRadius:r yRadius:r];
    if (fill) {
        [fill setFill];
        [path fill];
    }
    if (stroke && strokeW > 0) {
        [stroke setStroke];
        [path setLineWidth:strokeW];
        [path stroke];
    }
}

static void drawShadowRoundedBox(CGFloat x, CGFloat y, CGFloat w, CGFloat h, CGFloat r, NSColor *fill, NSColor *stroke, CGFloat strokeW, NSColor *shadowCol, CGFloat shadowBlur, CGFloat shadowDy) {
    [NSGraphicsContext saveGraphicsState];
    if (shadowCol && shadowBlur > 0) {
        NSShadow *sh = [[NSShadow alloc] init];
        sh.shadowColor = shadowCol;
        sh.shadowBlurRadius = shadowBlur;
        sh.shadowOffset = NSMakeSize(0, -shadowDy);
        [sh set];
    }
    drawRoundedBox(x, y, w, h, r, fill, nil, 0);
    [NSGraphicsContext restoreGraphicsState];
    if (stroke && strokeW > 0) {
        drawRoundedBox(x, y, w, h, r, nil, stroke, strokeW);
    }
}

static void drawCircle(CGFloat cx, CGFloat cy, CGFloat rad, NSColor *fill, NSColor *stroke, CGFloat strokeW) {
    NSBezierPath *path = [NSBezierPath bezierPathWithOvalInRect:TD(cx - rad, cy - rad, rad * 2.0, rad * 2.0)];
    if (fill) {
        [fill setFill];
        [path fill];
    }
    if (stroke && strokeW > 0) {
        [stroke setStroke];
        [path setLineWidth:strokeW];
        [path stroke];
    }
}

static void drawCroppedImageCard(NSImage *img, CGFloat x, CGFloat y, CGFloat w, CGFloat h, CGFloat radius, NSColor *borderColor, double alpha, BOOL shadow) {
    if (!img || alpha <= 0.01) return;
    if (shadow) {
        drawShadowRoundedBox(x, y, w, h, radius, rgba(20, 4, 18, 0.9 * alpha), nil, 0,
                             rgba(8, 2, 8, 0.58 * alpha), 44.0, 20.0);
    }
    [NSGraphicsContext saveGraphicsState];
    NSRect dst = TD(x, y, w, h);
    NSBezierPath *clip = [NSBezierPath bezierPathWithRoundedRect:dst xRadius:radius yRadius:radius];
    [clip addClip];

    CGSize imgSz = img.size;
    double scale = fmax(w / imgSz.width, h / imgSz.height);
    double sw = w / scale;
    double sh = h / scale;
    double sx = (imgSz.width - sw) * 0.5;
    double sy = (imgSz.height - sh) * 0.5;
    [img drawInRect:dst fromRect:NSMakeRect(sx, sy, sw, sh) operation:NSCompositingOperationSourceOver fraction:alpha];
    [NSGraphicsContext restoreGraphicsState];

    if (borderColor) {
        drawRoundedBox(x, y, w, h, radius, nil, borderColor, 2.0);
    }
}

// vImage Accelerate 180-degree shutter directional blur pass
static void applyDirectionalMotionBlur(NSBitmapImageRep *rep, int bx, int by) {
    if (!rep) return;
    if (bx <= 1 && by <= 1) return;
    uint32_t kx = (uint32_t)(fmin(25, bx) * 2 + 1);
    uint32_t ky = (uint32_t)(fmin(25, by) * 2 + 1);
    vImage_Buffer src = {
        .data = [rep bitmapData],
        .height = (vImagePixelCount)[rep pixelsHigh],
        .width = (vImagePixelCount)[rep pixelsWide],
        .rowBytes = (size_t)[rep bytesPerRow]
    };
    void *tmpData = malloc((size_t)[rep bytesPerRow] * (size_t)[rep pixelsHigh]);
    if (!tmpData) return;
    vImage_Buffer dst = {
        .data = tmpData,
        .height = src.height,
        .width = src.width,
        .rowBytes = src.rowBytes
    };
    uint8_t bg[4] = {0, 0, 0, 0};
    vImageBoxConvolve_ARGB8888(&src, &dst, NULL, 0, 0, ky, kx, bg, kvImageEdgeExtend);
    memcpy(src.data, dst.data, (size_t)[rep bytesPerRow] * (size_t)[rep pixelsHigh]);
    free(tmpData);
}

// ============================================================================
// 3D PROJECTION ENGINE (Ported from film/src/cinemaGraphics.tsx)
// ============================================================================
typedef struct {
    double sx;    // top-down X (0..1080)
    double sy;    // top-down Y (0..1920)
    double z;     // depth after rotation
    double scale;
} Proj3D;

static inline Proj3D project3D(double x, double y, double z, double rx, double ry, double rz, double cx, double cy, double fov) {
    double cosX = cos(rx), sinX = sin(rx);
    double y1 = y * cosX - z * sinX;
    double z1 = y * sinX + z * cosX;

    double cosY = cos(ry), sinY = sin(ry);
    double x2 = x * cosY + z1 * sinY;
    double z2 = -x * sinY + z1 * cosY;

    double cosZ = cos(rz), sinZ = sin(rz);
    double x3 = x2 * cosZ - y1 * sinZ;
    double y3 = x2 * sinZ + y1 * cosZ;

    double denom = fmax(120.0, fov - z2);
    double scale = fov / denom;
    Proj3D out = { cx + x3 * scale, cy + y3 * scale, z2, scale };
    return out;
}

typedef struct {
    double x0, y0;
    double x1, y1;
    double x2, y2;
    double x3, y3;
    double z;
    double u;
} Quad3D;

static int cmpQuadZ(const void *a, const void *b) {
    double za = ((const Quad3D *)a)->z;
    double zb = ((const Quad3D *)b)->z;
    if (za < zb) return -1;
    if (za > zb) return 1;
    return 0;
}

// ============================================================================
// PLATE BACKGROUNDS & 1. 3D ATMOSPHERIC HORIZON GRID
// ============================================================================
static void drawPlateBackground(BOOL lightPlate, double intensity, double crimsonTint) {
    if (lightPlate) {
        NSGradient *g = [[NSGradient alloc] initWithStartingColor:rgba(246, 238, 244, 1.0)
                                                      endingColor:rgba(238, 226, 236, 1.0)];
        [g drawInRect:NSMakeRect(0, 0, W, H) angle:-90.0];
        NSGradient *warmBloom = [[NSGradient alloc] initWithStartingColor:rgba(251, 179, 3, 0.16 * intensity)
                                                              endingColor:rgba(246, 238, 244, 0.0)];
        [warmBloom drawFromCenter:P(CX, CY - 80) radius:0.0 toCenter:P(CX, CY - 80) radius:760.0 options:0];
    } else {
        NSGradient *g = [[NSGradient alloc] initWithStartingColor:rgba(34, 6, 28, 1.0)
                                                      endingColor:rgba(16, 3, 14, 1.0)];
        [g drawInRect:NSMakeRect(0, 0, W, H) angle:-90.0];
        NSGradient *plumBloom = [[NSGradient alloc] initWithStartingColor:rgba(92, 18, 76, 0.42 * intensity)
                                                              endingColor:rgba(34, 6, 28, 0.0)];
        [plumBloom drawFromCenter:P(CX, 460) radius:0.0 toCenter:P(CX, 460) radius:780.0 options:0];

        if (crimsonTint > 0.01) {
            NSGradient *crit = [[NSGradient alloc] initWithStartingColor:rgba(255, 80, 70, 0.24 * crimsonTint)
                                                             endingColor:rgba(34, 6, 28, 0.0)];
            [crit drawFromCenter:P(CX, CY - 120) radius:0.0 toCenter:P(CX, CY - 120) radius:740.0 options:0];
        } else {
            NSGradient *amb = [[NSGradient alloc] initWithStartingColor:rgba(251, 179, 3, 0.20 * intensity)
                                                            endingColor:rgba(34, 6, 28, 0.0)];
            [amb drawFromCenter:P(CX, CY + 240) radius:0.0 toCenter:P(CX, CY + 240) radius:740.0 options:0];
        }
    }
}

static void drawHorizonGrid3D(int frame, BOOL lightPlate, double intensity, double horizonOffsetY) {
    double t = frame / FPS;
    double rms = frameRms(frame);
    double pk = framePeak(frame);
    double horizonCy = CY + 220 + horizonOffsetY;

    // Volumetric horizon glow reactive to audio RMS/peak
    NSGradient *rad = nil;
    if (lightPlate) {
        rad = [[NSGradient alloc] initWithColorsAndLocations:
            rgba(251, 179, 3, (0.20 + rms * 0.14) * intensity), 0.0,
            rgba(216, 169, 214, 0.14 * intensity), 0.45,
            rgba(246, 238, 244, 0.0), 1.0, nil];
    } else {
        rad = [[NSGradient alloc] initWithColorsAndLocations:
            rgba(251, 179, 3, (0.24 + rms * 0.18) * intensity), 0.0,
            rgba(92, 18, 76, (0.34 + pk * 0.10) * intensity), 0.42,
            rgba(24, 4, 20, 0.0), 1.0, nil];
    }
    double gx = CX + sin(t * 0.55) * 70.0;
    [rad drawFromCenter:P(gx, horizonCy) radius:0.0 toCenter:P(gx, horizonCy) radius:680.0 options:0];

    double zSpeed = fmod(t * 250.0, 120.0);
    for (int iz = 1; iz <= 15; iz++) {
        double z = iz * 120.0 - zSpeed - 300.0;
        Proj3D pL = project3D(-1500.0, 380.0, z, 0.18, 0.0, 0.0, CX, horizonCy - 180.0, 900.0);
        Proj3D pR = project3D(1500.0, 380.0, z, 0.18, 0.0, 0.0, CX, horizonCy - 180.0, 900.0);
        double a = clamp01(1.0 - iz / 15.0) * 0.52 * intensity;
        NSColor *col = lightPlate
            ? rgba(42, 8, 35, 0.09 * a)
            : rgba(251, 179, 3, (0.14 + rms * 0.08) * a);
        [col setStroke];
        NSBezierPath *line = [NSBezierPath bezierPath];
        [line setLineWidth:1.3];
        [line moveToPoint:P(pL.sx, pL.sy)];
        [line lineToPoint:P(pR.sx, pR.sy)];
        [line stroke];
    }

    for (int ix = -10; ix <= 10; ix++) {
        double x = ix * 165.0;
        Proj3D pNear = project3D(x, 380.0, 420.0, 0.18, 0.0, 0.0, CX, horizonCy - 180.0, 900.0);
        Proj3D pFar = project3D(x, 380.0, -1450.0, 0.18, 0.0, 0.0, CX, horizonCy - 180.0, 900.0);
        double a = (1.0 - fabs((double)ix) / 11.0) * 0.34 * intensity;
        NSColor *col = lightPlate
            ? rgba(42, 8, 35, 0.08 * a)
            : rgba(251, 179, 3, (0.12 + rms * 0.06) * a);
        [col setStroke];
        NSBezierPath *line = [NSBezierPath bezierPath];
        [line setLineWidth:1.1];
        [line moveToPoint:P(pNear.sx, pNear.sy)];
        [line lineToPoint:P(pFar.sx, pFar.sy)];
        [line stroke];
    }
}

// ============================================================================
// 2. HALFTONE DOT-RING IRIS TRANSITION (kinetic.html / SaywhenDemo.tsx)
// ============================================================================
static void beginHalftoneIrisClip(double radius) {
    NSBezierPath *clip = [NSBezierPath bezierPathWithOvalInRect:TD(CX - radius, CY - radius, radius * 2.0, radius * 2.0)];
    [clip addClip];
}

static void drawHalftoneIrisRing(double radius, double band, double opacity) {
    if (opacity <= 0.01) return;
    double p = 15.0;
    double a = 0.35;
    double ca = cos(a), sa = sin(a);
    double Rm = radius + band;
    int n = (int)ceil(Rm / p) + 1;

    [rgba(251, 179, 3, opacity) setFill];
    NSBezierPath *dots = [NSBezierPath bezierPath];
    for (int i = -n; i <= n; i++) {
        for (int j = -n; j <= n; j++) {
            double gx = i * p;
            double gy = j * p;
            double X = CX + gx * ca - gy * sa;
            double Y = CY + gx * sa + gy * ca;
            if (X < -16.0 || X > W + 16.0 || Y < -16.0 || Y > H + 16.0) continue;
            double d = hypot(X - CX, Y - CY);
            if (d < radius || d > Rm) continue;
            double r = p * 0.46 * pow(1.0 - (d - radius) / band, 0.8);
            if (r < 0.4) continue;
            [dots appendBezierPathWithOvalInRect:TD(X - r, Y - r, r * 2.0, r * 2.0)];
        }
    }
    [dots fill];
}

// ============================================================================
// 3. TILTED WHITE PROBLEM CARD + HAND-DRAWN RED ROUGHRECT & STAMP
// ============================================================================
static NSBezierPath *buildRoughRectPath(double x0, double y0, double w, double h, uint32_t seed) {
    uint32_t sd = seed;
    const double j = 6.0;
    double rx0 = x0 + (lcgFloat(&sd) - 0.5) * j;
    double ry0 = y0 + (lcgFloat(&sd) - 0.5) * j;
    double rx1 = x0 + w + (lcgFloat(&sd) - 0.5) * j;
    double ry1 = y0 + h + (lcgFloat(&sd) - 0.5) * j;

    NSBezierPath *p = [NSBezierPath bezierPath];
    [p moveToPoint:P(rx0 + 14, ry0 + (lcgFloat(&sd) - 0.5) * 4.0)];
    [p curveToPoint:P(rx1, ry0 + (lcgFloat(&sd) - 0.5) * j)
      controlPoint1:P(x0 + w * 0.35, ry0 - 5.0 + (lcgFloat(&sd) - 0.5) * j)
      controlPoint2:P(x0 + w * 0.70, ry0 + 4.0 + (lcgFloat(&sd) - 0.5) * j)];
    [p curveToPoint:P(rx1 + (lcgFloat(&sd) - 0.5) * j, ry1)
      controlPoint1:P(rx1 + 4.0 + (lcgFloat(&sd) - 0.5) * j, y0 + h * 0.38)
      controlPoint2:P(rx1 - 3.0 + (lcgFloat(&sd) - 0.5) * j, y0 + h * 0.70)];
    [p curveToPoint:P(rx0, ry1 + (lcgFloat(&sd) - 0.5) * j)
      controlPoint1:P(x0 + w * 0.66, ry1 + 5.0 + (lcgFloat(&sd) - 0.5) * j)
      controlPoint2:P(x0 + w * 0.30, ry1 - 4.0 + (lcgFloat(&sd) - 0.5) * j)];
    [p curveToPoint:P(rx0 - 4, ry0 + 16)
      controlPoint1:P(rx0 - 4.0 + (lcgFloat(&sd) - 0.5) * j, y0 + h * 0.62)
      controlPoint2:P(rx0 + 4.0 + (lcgFloat(&sd) - 0.5) * j, y0 + h * 0.30)];
    return p;
}

static void strokePartialPath(NSBezierPath *path, double frac, CGFloat lineW, NSColor *col) {
    if (frac <= 0.01) return;
    [NSGraphicsContext saveGraphicsState];
    CGFloat dash[2] = { frac * 2800.0, 4000.0 };
    [path setLineDash:dash count:2 phase:0.0];
    [path setLineCapStyle:NSLineCapStyleRound];
    [path setLineJoinStyle:NSLineJoinStyleRound];
    [path setLineWidth:lineW];
    [col setStroke];
    [path stroke];
    [NSGraphicsContext restoreGraphicsState];
}

static void drawTiltedProblemCard(CGFloat cx, CGFloat cy, CGFloat w, CGFloat h, double rotDeg,
                                  NSString *tagText, NSString *cmdText, NSString *subText,
                                  NSString *stampText, double enterU, double penU, uint32_t seed) {
    if (enterU <= 0.01) return;
    [NSGraphicsContext saveGraphicsState];

    NSAffineTransform *xf = [NSAffineTransform transform];
    [xf translateXBy:cx yBy:H - cy + (1.0 - enterU) * -32.0];
    [xf rotateByDegrees:rotDeg];
    double sc = lerpd(0.92, 1.0, enterU);
    [xf scaleBy:sc];
    [xf concat];

    CGFloat lx = -w * 0.5;
    CGFloat by = -h * 0.5;

    // Deep drop shadow + crisp white card
    NSShadow *sh = [[NSShadow alloc] init];
    sh.shadowColor = rgba(0, 0, 0, 0.42 * enterU);
    sh.shadowBlurRadius = 38.0;
    sh.shadowOffset = NSMakeSize(0, -18.0);
    [NSGraphicsContext saveGraphicsState];
    [sh set];
    [rgba(255, 255, 255, enterU) setFill];
    [[NSBezierPath bezierPathWithRoundedRect:NSMakeRect(lx, by, w, h) xRadius:22 yRadius:22] fill];
    [NSGraphicsContext restoreGraphicsState];

    // Tag text
    [tagText drawAtPoint:NSMakePoint(lx + 34, by + h - 46)
          withAttributes:@{
              NSFontAttributeName: fontMono(16, 700),
              NSForegroundColorAttributeName: rgba(109, 86, 105, enterU),
              NSKernAttributeName: @(1.6)
          }];

    // Command pill inside card
    [rgba(246, 238, 244, enterU) setFill];
    [rgba(34, 6, 28, 0.10 * enterU) setStroke];
    NSBezierPath *pill = [NSBezierPath bezierPathWithRoundedRect:NSMakeRect(lx + 32, by + 74, w - 64, 74) xRadius:14 yRadius:14];
    [pill fill];
    [pill setLineWidth:1.2];
    [pill stroke];

    [cmdText drawAtPoint:NSMakePoint(lx + 52, by + 91)
          withAttributes:@{
              NSFontAttributeName: fontMono(32, 700),
              NSForegroundColorAttributeName: rgba(42, 8, 35, enterU),
              NSKernAttributeName: @(-0.8)
          }];

    // Sub caption
    [subText drawAtPoint:NSMakePoint(lx + 34, by + 26)
          withAttributes:@{
              NSFontAttributeName: fontSans(21, 500),
              NSForegroundColorAttributeName: rgba(109, 86, 105, enterU)
          }];

    // Hand-drawn roughRect red cross-out perimeter in local Cocoa coords
    if (penU > 0.01) {
        uint32_t sd = seed;
        const double j = 5.0;
        double mx = -14.0;
        double rx0 = lx + mx + (lcgFloat(&sd) - 0.5) * j;
        double ry0 = by + mx + (lcgFloat(&sd) - 0.5) * j;
        double rw = w - mx * 2.0;
        double rh = h - mx * 2.0;
        double rx1 = rx0 + rw;
        double ry1 = ry0 + rh;

        NSBezierPath *rp = [NSBezierPath bezierPath];
        [rp moveToPoint:NSMakePoint(rx0 + 12, ry1)];
        [rp curveToPoint:NSMakePoint(rx1, ry1 + (lcgFloat(&sd) - 0.5) * j)
           controlPoint1:NSMakePoint(lx + w * 0.3, ry1 + 5)
           controlPoint2:NSMakePoint(lx + w * 0.7, ry1 - 4)];
        [rp curveToPoint:NSMakePoint(rx1 + (lcgFloat(&sd) - 0.5) * j, ry0)
           controlPoint1:NSMakePoint(rx1 + 5, by + h * 0.65)
           controlPoint2:NSMakePoint(rx1 - 3, by + h * 0.30)];
        [rp curveToPoint:NSMakePoint(rx0, ry0 + (lcgFloat(&sd) - 0.5) * j)
           controlPoint1:NSMakePoint(lx + w * 0.68, ry0 - 5)
           controlPoint2:NSMakePoint(lx + w * 0.25, ry0 + 4)];
        [rp curveToPoint:NSMakePoint(rx0 - 4, ry1 - 14)
           controlPoint1:NSMakePoint(rx0 - 4, by + h * 0.35)
           controlPoint2:NSMakePoint(rx0 + 4, by + h * 0.70)];

        strokePartialPath(rp, penU, 5.5, rgba(255, 107, 94, enterU));

        // Tilted red stamp pill overlapping bottom-right corner
        NSFont *sf = fontMono(17, 700);
        CGSize ssz = measureText(stampText, sf, 0.2);
        CGFloat sw = ssz.width + 34.0;
        CGFloat sh2 = 38.0;

        [NSGraphicsContext saveGraphicsState];
        NSAffineTransform *stXf = [NSAffineTransform transform];
        [stXf translateXBy:lx + w - 24 - sw * 0.5 yBy:by - 2.0];
        [stXf rotateByDegrees:-4.5];
        [stXf scaleBy:lerpd(0.65, 1.0, bo(penU))];
        [stXf concat];

        [rgba(255, 107, 94, penU * enterU) setFill];
        [[NSBezierPath bezierPathWithRoundedRect:NSMakeRect(-sw * 0.5, -sh2 * 0.5, sw, sh2) xRadius:19 yRadius:19] fill];
        [stampText drawAtPoint:NSMakePoint(-ssz.width * 0.5, -ssz.height * 0.5 + 1.0)
                withAttributes:@{
                    NSFontAttributeName: sf,
                    NSForegroundColorAttributeName: rgba(255, 255, 255, penU * enterU),
                    NSKernAttributeName: @(0.2)
                }];
        [NSGraphicsContext restoreGraphicsState];
    }

    [NSGraphicsContext restoreGraphicsState];
}

// ============================================================================
// 4. 3D SHATTERING KEYCAPS PRISM EXPLOSION (ShatteringKeycaps3DCanvas)
// ============================================================================
static void drawShatteringKeycaps3D(double boom, double fade, double cx, double cy) {
    if (boom <= 0.01 || fade <= 0.01) return;

    // Expanding Amber Shockwave Ring
    if (boom > 0.0 && boom < 1.0) {
        double r = lerpd(40.0, 920.0, boom);
        NSBezierPath *ring = [NSBezierPath bezierPathWithOvalInRect:TD(cx - r, cy - r, r * 2.0, r * 2.0)];
        [rgba(251, 179, 3, (1.0 - boom) * 0.88 * fade) setStroke];
        [ring setLineWidth:lerpd(18.0, 1.5, boom)];
        [ring stroke];
    }

    uint32_t sd = 9001u;
    for (int i = 0; i < 84; i++) {
        double originX = (i % 3 == 0 ? -240.0 : (i % 3 == 1 ? 0.0 : 240.0)) + (lcgFloat(&sd) - 0.5) * 320.0;
        double originY = (i % 3 == 0 ? -260.0 : (i % 3 == 1 ? 0.0 : 260.0)) + (lcgFloat(&sd) - 0.5) * 140.0;
        double vx = originX * (1.8 + lcgFloat(&sd) * 1.4) + (lcgFloat(&sd) - 0.5) * 620.0;
        double vy = originY * 1.8 - 160.0 + lcgFloat(&sd) * 560.0;
        double vz = 240.0 + lcgFloat(&sd) * 700.0;

        double x = lerpd(originX, originX + vx, boom);
        double y = lerpd(originY, originY + vy + 320.0 * boom * boom, boom);
        double z = lerpd(0.0, vz, boom);

        double rx = boom * (3.0 + lcgFloat(&sd) * 8.0);
        double ry = boom * (3.0 + lcgFloat(&sd) * 8.0);
        double rz = boom * (2.0 + lcgFloat(&sd) * 6.0);
        double sz = 16.0 + lcgFloat(&sd) * 26.0;

        Proj3D c0 = project3D(x - sz, y - sz, z, rx, ry, rz, cx, cy, 980.0);
        Proj3D c1 = project3D(x + sz, y - sz, z, rx, ry, rz, cx, cy, 980.0);
        Proj3D c2 = project3D(x + sz, y + sz, z, rx, ry, rz, cx, cy, 980.0);
        Proj3D c3 = project3D(x - sz, y + sz, z, rx, ry, rz, cx, cy, 980.0);

        double a = fade * (0.45 + 0.55 * lcgFloat(&sd));
        NSColor *col = (i % 4 == 0) ? rgba(251, 179, 3, a)
                     : (i % 4 == 1) ? rgba(255, 107, 94, a)
                     : rgba(223, 229, 236, a);
        [col setFill];
        NSBezierPath *shard = [NSBezierPath bezierPath];
        [shard moveToPoint:P(c0.sx, c0.sy)];
        [shard lineToPoint:P(c1.sx, c1.sy)];
        [shard lineToPoint:P(c2.sx, c2.sy)];
        [shard lineToPoint:P(c3.sx, c3.sy)];
        [shard closePath];
        [shard fill];
    }
}

// ============================================================================
// 5. 3D PARAMETRIC S-RIBBON SCULPTURE (Torus Iris -> Double-Helix -> Launch)
// ============================================================================
static void drawParametricRibbon3D(int frame, double t, double m01, double m12, double cx, double cy, double scaleFactor) {
    const int N = 220;
    double rms = frameRms(frame);

    typedef struct {
        double x, y, z;
        double nx, ny, nz;
        double u;
    } RibPt;

    RibPt pts[221];
    for (int i = 0; i <= N; i++) {
        double u = (double)i / (double)N;
        double ang = u * M_PI * 4.0;

        // Stage 0 ("Look."): 3D Torus Iris Ring
        double r0 = (235.0 + 44.0 * cos(u * M_PI * 6.0 + t * 3.0)) * scaleFactor;
        double x0 = r0 * cos(u * M_PI * 2.0);
        double y0 = r0 * sin(u * M_PI * 2.0);
        double z0 = 90.0 * sin(u * M_PI * 4.0 + t * 2.0) * scaleFactor;

        // Stage 1 ("Talk."): 3D Acoustic Double-Helix Wave
        double x1 = (u - 0.5) * 920.0 * scaleFactor;
        double env = sin(u * M_PI);
        double amp = (135.0 + rms * 130.0) * env * scaleFactor;
        double y1 = amp * sin(ang * 2.2 - t * 9.0);
        double z1 = amp * cos(ang * 2.2 - t * 9.0);

        // Stage 2 ("Sent."): 3D Folded S-Ribbon Launch Vector
        double sCurve = sin((u - 0.5) * M_PI * 2.0);
        double x2 = sCurve * 310.0 * scaleFactor;
        double y2 = (0.5 - u) * 440.0 * scaleFactor;
        double z2 = cos(u * M_PI * 2.0) * 155.0 * scaleFactor;

        double xa = lerpd(x0, x1, m01);
        double ya = lerpd(y0, y1, m01);
        double za = lerpd(z0, z1, m01);

        double twist = u * M_PI * 2.0 + t * 1.4;
        double w = 32.0 * scaleFactor * (0.45 + 0.55 * sin(u * M_PI));

        pts[i].x = lerpd(xa, x2, m12);
        pts[i].y = lerpd(ya, y2, m12);
        pts[i].z = lerpd(za, z2, m12);
        pts[i].nx = cos(twist) * w;
        pts[i].ny = sin(twist) * w;
        pts[i].nz = cos(twist * 0.7) * w * 0.5;
        pts[i].u = u;
    }

    Quad3D quads[220];
    double rx = 0.22 * sin(t * 0.9);
    double ry = 0.35 * cos(t * 0.7);
    double rz = 0.08 * sin(t * 0.5);

    for (int i = 0; i < N; i++) {
        RibPt a = pts[i];
        RibPt b = pts[i + 1];
        Proj3D aL = project3D(a.x - a.nx, a.y - a.ny, a.z - a.nz, rx, ry, rz, cx, cy, 1050.0);
        Proj3D aR = project3D(a.x + a.nx, a.y + a.ny, a.z + a.nz, rx, ry, rz, cx, cy, 1050.0);
        Proj3D bR = project3D(b.x + b.nx, b.y + b.ny, b.z + b.nz, rx, ry, rz, cx, cy, 1050.0);
        Proj3D bL = project3D(b.x - b.nx, b.y - b.ny, b.z - b.nz, rx, ry, rz, cx, cy, 1050.0);

        quads[i].x0 = aL.sx; quads[i].y0 = aL.sy;
        quads[i].x1 = aR.sx; quads[i].y1 = aR.sy;
        quads[i].x2 = bR.sx; quads[i].y2 = bR.sy;
        quads[i].x3 = bL.sx; quads[i].y3 = bL.sy;
        quads[i].z = (aL.z + bR.z) * 0.5;
        quads[i].u = a.u;
    }

    qsort(quads, (size_t)N, sizeof(Quad3D), cmpQuadZ);

    double morphMix = clamp01(m01 + m12);
    for (int i = 0; i < N; i++) {
        Quad3D q = quads[i];
        double torusArc = 0.5 - 0.5 * cos(q.u * M_PI * 2.0);
        double helixRamp = eio(seg(q.u, 0.52, 0.86));
        double tipGrad = lerpd(torusArc * 0.72, helixRamp, morphMix);
        double amberMix = clamp01(tipGrad * (0.65 + 0.35 * m12) + m12 * 0.78);
        double shade = clampd((q.z + 260.0) / 520.0, 0.25, 1.0);

        double silV = lerpd(75.0, 224.0, shade);
        double ambR = lerpd(180.0, 251.0, shade);
        double ambG = lerpd(95.0, 179.0, shade);
        double ambB = lerpd(8.0, 3.0, shade);

        double r = lerpd(silV, ambR, amberMix);
        double g = lerpd(silV * 0.94, ambG, amberMix);
        double b = lerpd(silV, ambB, amberMix);
        double a = lerpd(0.84, 0.92, amberMix);

        NSColor *col = rgba(r, g, b, a);
        [col setFill];
        [col setStroke];
        NSBezierPath *qp = [NSBezierPath bezierPath];
        [qp moveToPoint:P(q.x0, q.y0)];
        [qp lineToPoint:P(q.x1, q.y1)];
        [qp lineToPoint:P(q.x2, q.y2)];
        [qp lineToPoint:P(q.x3, q.y3)];
        [qp closePath];
        [qp fill];
        [qp setLineWidth:0.8];
        [qp stroke];
    }
}

// ============================================================================
// 6. 3D 478-POINT BIOMETRIC FACE TOPOLOGY & VOLUMETRIC GAZE CONE
// ============================================================================
static void drawBiometricFaceMesh3D(double t, double rms, double lockU, double extraYaw,
                                    double faceCx, double faceCy, double scaleFactor,
                                    double coneTargetY, double coneHalfW) {
    double yaw = lerpd(-0.54, -0.024, lockU) + 0.02 * sin(t * 2.1) + extraYaw;
    double pitch = lerpd(0.22, 0.015, lockU) + 0.015 * cos(t * 1.7);
    double roll = lerpd(-0.12, 0.005, lockU);

    // 1. Volumetric Gaze Cone Frustum projecting from both eyes downward onto the UI window
    if (lockU > 0.35 && coneTargetY > faceCy) {
        double coneAlpha = seg(lockU, 0.35, 0.95) * (0.28 + rms * 0.18);
        Proj3D leftEye = project3D(-68.0 * scaleFactor, -42.0 * scaleFactor, 165.0 * scaleFactor,
                                   pitch, yaw, roll, faceCx, faceCy, 950.0);
        Proj3D rightEye = project3D(68.0 * scaleFactor, -42.0 * scaleFactor, 165.0 * scaleFactor,
                                    pitch, yaw, roll, faceCx, faceCy, 950.0);
        Proj3D eyes[2] = { leftEye, rightEye };
        for (int e = 0; e < 2; e++) {
            NSBezierPath *cone = [NSBezierPath bezierPath];
            [cone moveToPoint:P(eyes[e].sx, eyes[e].sy)];
            [cone lineToPoint:P(faceCx - coneHalfW, coneTargetY)];
            [cone lineToPoint:P(faceCx + coneHalfW, coneTargetY)];
            [cone closePath];

            [NSGraphicsContext saveGraphicsState];
            [cone addClip];
            NSGradient *cg = [[NSGradient alloc] initWithColorsAndLocations:
                rgba(251, 179, 3, coneAlpha * 1.45), 0.0,
                rgba(184, 204, 228, coneAlpha * 0.65), 0.52,
                rgba(251, 179, 3, 0.0), 1.0, nil];
            [cg drawFromPoint:P(eyes[e].sx, eyes[e].sy)
                      toPoint:P(faceCx, coneTargetY)
                      options:0];
            [NSGraphicsContext restoreGraphicsState];
        }
    }

    // 2. 16 x 20 3D Parameterized Facial Mesh
    const int ROWS = 16;
    const int COLS = 20;
    typedef struct {
        double sx, sy, z;
        int region; // 0=skin, 1=eye, 2=lips
    } FaceVtx;
    FaceVtx grid[17][21];

    for (int r = 0; r <= ROWS; r++) {
        double v = (double)r / (double)ROWS;
        double phi = (v - 0.5) * M_PI * 0.88;
        for (int c = 0; c <= COLS; c++) {
            double u = (double)c / (double)COLS;
            double theta = (u - 0.5) * M_PI * 0.92;

            double jawTaper = 1.0 - 0.28 * pow(fmax(0.0, v - 0.45), 1.4);
            double x = 185.0 * sin(theta) * cos(phi) * jawTaper * scaleFactor;
            double y = 245.0 * sin(phi) * scaleFactor;
            double z = 175.0 * cos(theta) * cos(phi) * scaleFactor;

            double noseDistX = fabs(u - 0.5);
            if (v > 0.36 && v < 0.64 && noseDistX < 0.12) {
                z += (1.0 - noseDistX / 0.12) * 38.0 * sin(((v - 0.36) / 0.28) * M_PI) * scaleFactor;
            }

            double eyeDistL = hypot(u - 0.33, v - 0.40);
            double eyeDistR = hypot(u - 0.67, v - 0.40);
            if (eyeDistL < 0.10 || eyeDistR < 0.10) z -= 18.0 * scaleFactor;

            double mouthDist = hypot((u - 0.50) * 1.4, (v - 0.74) * 2.2);
            int region = 0;
            if (eyeDistL < 0.095 || eyeDistR < 0.095) region = 1;
            if (mouthDist < 0.18) {
                region = 2;
                double lipOpen = (0.25 + rms * 0.95) * 18.0 * scaleFactor;
                y += (v > 0.74 ? 1.0 : -1.0) * lipOpen * (1.0 - mouthDist / 0.18);
                z += 14.0 * scaleFactor;
            }

            Proj3D p = project3D(x, y, z, pitch, yaw, roll, faceCx, faceCy, 950.0);
            grid[r][c] = (FaceVtx){ p.sx, p.sy, p.z, region };
        }
    }

    // Wireframe triangulation
    NSColor *skinWire = rgba(184, 204, 228, 0.22 + 0.18 * lockU);
    NSColor *hotWire = rgba(251, 179, 3, 0.60 + 0.35 * lockU);
    NSBezierPath *skinPath = [NSBezierPath bezierPath];
    NSBezierPath *hotPath = [NSBezierPath bezierPath];

    for (int r = 0; r < ROWS; r++) {
        for (int c = 0; c < COLS; c++) {
            FaceVtx p00 = grid[r][c];
            FaceVtx p10 = grid[r + 1][c];
            FaceVtx p01 = grid[r][c + 1];
            FaceVtx p11 = grid[r + 1][c + 1];
            BOOL isHot = (p00.region != 0 || p11.region != 0);
            NSBezierPath *tp = isHot ? hotPath : skinPath;
            [tp moveToPoint:P(p00.sx, p00.sy)];
            [tp lineToPoint:P(p01.sx, p01.sy)];
            [tp lineToPoint:P(p11.sx, p11.sy)];
            [tp lineToPoint:P(p00.sx, p00.sy)];
            [tp lineToPoint:P(p10.sx, p10.sy)];
            [tp lineToPoint:P(p11.sx, p11.sy)];
        }
    }
    [skinWire setStroke];
    [skinPath setLineWidth:0.9 * scaleFactor];
    [skinPath stroke];

    [hotWire setStroke];
    [hotPath setLineWidth:1.65 * scaleFactor];
    [hotPath stroke];

    // Landmark nodes
    NSBezierPath *skinDots = [NSBezierPath bezierPath];
    NSBezierPath *hotDots = [NSBezierPath bezierPath];
    for (int r = 0; r <= ROWS; r++) {
        for (int c = 0; c <= COLS; c++) {
            FaceVtx p = grid[r][c];
            double rad = (p.region != 0 ? 3.2 : 1.7) * scaleFactor;
            NSBezierPath *dp = (p.region != 0) ? hotDots : skinDots;
            [dp appendBezierPathWithOvalInRect:TD(p.sx - rad, p.sy - rad, rad * 2.0, rad * 2.0)];
        }
    }
    [rgba(223, 229, 236, 0.50) setFill];
    [skinDots fill];
    [rgba(251, 179, 3, 0.96) setFill];
    [hotDots fill];

    // Rotating Biometric Target Reticle
    [NSGraphicsContext saveGraphicsState];
    double retR = 290.0 * scaleFactor;
    NSBezierPath *ret = [NSBezierPath bezierPathWithOvalInRect:TD(faceCx - retR, faceCy - retR, retR * 2.0, retR * 2.0)];
    CGFloat dash[2] = { 18.0 * scaleFactor, 12.0 * scaleFactor };
    [ret setLineDash:dash count:2 phase:t * 38.0];
    [ret setLineWidth:2.2];
    [(lockU > 0.55 ? C_AMBER : C_ARMED) setStroke];
    [ret stroke];
    [NSGraphicsContext restoreGraphicsState];
}

// ============================================================================
// 7. 3D ACOUSTIC FREQUENCY WATERFALL TERRAIN
// ============================================================================
static void drawAcousticTerrain3D(int frame, double baseY, BOOL lightPlate, double intensity) {
    const int ROWS = 24;
    const int COLS = 56;
    double terrainCy = baseY - 120.0;

    for (int r = ROWS - 1; r >= 0; r--) {
        int hf = frame - r * 2;
        if (hf < 0) hf = 0;
        double env = frameRms(hf);
        double pk = framePeak(hf);
        double z = 260.0 - r * 58.0;

        NSBezierPath *ridge = [NSBezierPath bezierPath];
        for (int c = 0; c <= COLS; c++) {
            double u = (double)c / (double)COLS;
            double x = (u - 0.5) * 1420.0;
            double bell = exp(-pow((u - 0.5) * 2.9, 2.0));
            double wave = sin(u * 22.0 + hf * 0.28) * 0.55 + cos(u * 38.0 - hf * 0.19) * 0.45;
            double amp = (28.0 + env * 145.0 + pk * 65.0) * bell;
            double y = 260.0 - fabs(wave) * amp;

            Proj3D p = project3D(x, y, z, 0.18, 0.0, 0.0, CX, terrainCy, 950.0);
            if (c == 0) [ridge moveToPoint:P(p.sx, p.sy)];
            else [ridge lineToPoint:P(p.sx, p.sy)];
        }
        double alpha = clampd(1.0 - (double)r / (double)ROWS, 0.08, 0.92) * intensity;
        NSColor *col = (r < 5)
            ? rgba(251, 179, 3, alpha)
            : (lightPlate ? rgba(42, 8, 35, alpha * 0.36) : rgba(223, 229, 236, alpha * 0.28));
        [col setStroke];
        [ridge setLineWidth:(r == 0 ? 3.2 : 1.5)];
        [ridge stroke];
    }
}

// ============================================================================
// 8. WET-INK WORD STREAM TYPOGRAPHY (words arrive in #fbb303 -> cool to ink)
// ============================================================================
static void drawWetInkLines(NSArray<NSString *> *lines, double t, double t0, double cps, double settle,
                            CGFloat x, CGFloat y, CGFloat lineH, NSFont *font,
                            double inkR, double inkG, double inkB,
                            double accR, double accG, double accB, BOOL showCursor) {
    NSUInteger totalLen = 0;
    for (NSString *ln in lines) totalLen += ln.length;
    NSInteger revealedChars = (NSInteger)floor(clampd((t - t0) * cps, 0.0, (double)totalLen));

    NSInteger globalIdx = 0;
    CGFloat lastCursorX = x;
    CGFloat lastCursorY = y;

    for (NSUInteger li = 0; li < lines.count; li++) {
        NSString *line = lines[li];
        CGFloat cx = x;
        CGFloat cy = y + (CGFloat)li * lineH;
        NSUInteger i = 0;

        while (i < line.length && globalIdx < revealedChars) {
            NSRange ws = [line rangeOfCharacterFromSet:[NSCharacterSet whitespaceCharacterSet]
                                               options:0
                                                 range:NSMakeRange(i, line.length - i)];
            NSUInteger wEnd = (ws.location == NSNotFound) ? line.length : (ws.location + 1);
            NSUInteger wLen = wEnd - i;
            NSInteger avail = revealedChars - globalIdx;
            NSUInteger drawLen = (avail >= (NSInteger)wLen) ? wLen : (NSUInteger)avail;
            NSString *token = [line substringWithRange:NSMakeRange(i, drawLen)];

            double doneTime = t0 + (double)(globalIdx + wLen) / cps;
            double p = seg(t, doneTime + 0.06, doneTime + 0.06 + settle);
            double cr = lerpd(accR, inkR, p);
            double cg = lerpd(accG, inkG, p);
            double cb = lerpd(accB, inkB, p);

            drawText(token, cx, cy, font, rgba(cr, cg, cb, 1.0), -0.8);
            CGSize tsz = measureText(token, font, -0.8);
            cx += tsz.width;
            lastCursorX = cx;
            lastCursorY = cy;

            globalIdx += (NSInteger)wLen;
            i = wEnd;
        }
    }

    if (showCursor && revealedChars > 0) {
        double blink = ((int)floor(t * 2.8) % 2 == 0) ? 1.0 : 0.25;
        drawRoundedBox(lastCursorX + 5, lastCursorY + 6, 4.5, font.pointSize * 1.05, 2.0,
                       rgba(251, 179, 3, blink), nil, 0);
    }
}

// ============================================================================
// 9. HYPERSPACE 3D WARP & 165-SQUARE PIXEL BURST
// ============================================================================
static void drawLightSpeedWarp(double t, double t0, double cx, double cy) {
    double warpU = seg(t, t0, t0 + 0.95);
    if (warpU <= 0.01 || warpU >= 0.99) return;
    double fade = 1.0 - seg(t, t0 + 0.55, t0 + 0.95);

    uint32_t sd = 4242u;
    for (int i = 0; i < 160; i++) {
        double ang = lcgFloat(&sd) * M_PI * 2.0;
        double rad = 50.0 + lcgFloat(&sd) * 640.0;
        double z0 = -800.0 + lcgFloat(&sd) * 1150.0;
        double z1 = z0 + warpU * 1450.0;
        double tailZ = z1 - 240.0 * (1.0 - warpU * 0.4);

        double x = cos(ang) * rad;
        double y = sin(ang) * rad;
        Proj3D pHead = project3D(x, y, z1, 0, 0, 0, cx, cy, 900.0);
        Proj3D pTail = project3D(x, y, tailZ, 0, 0, 0, cx, cy, 900.0);

        double a = fade * (0.35 + 0.65 * lcgFloat(&sd));
        NSColor *col = (i % 3 == 0) ? rgba(251, 179, 3, a) : rgba(223, 229, 236, a);
        [col setStroke];
        NSBezierPath *st = [NSBezierPath bezierPath];
        [st setLineWidth:(i % 4 == 0 ? 3.2 : 1.6)];
        [st moveToPoint:P(pTail.sx, pTail.sy)];
        [st lineToPoint:P(pHead.sx, pHead.sy)];
        [st stroke];
    }
}

static void drawPixelBurst(double t, double t0, double cx, double cy) {
    double u = seg(t, t0, t0 + 0.52);
    if (u <= 0.01 || u >= 0.99) return;
    double fade = 1.0 - seg(t, t0 + 0.25, t0 + 0.52);

    uint32_t sd = 7771u;
    for (int i = 0; i < 120; i++) {
        double ang = lcgFloat(&sd) * M_PI * 2.0;
        double dist = (60.0 + lcgFloat(&sd) * 480.0) * eo(u);
        double sz = lerpd(14.0, 4.0, u) * (0.5 + lcgFloat(&sd));
        double px = cx + cos(ang) * dist;
        double py = cy + sin(ang) * dist * 0.85;
        NSColor *col = (i % 3 == 0) ? rgba(251, 179, 3, fade) :
                       (i % 3 == 1) ? rgba(255, 201, 77, fade * 0.9) : rgba(223, 229, 236, fade * 0.85);
        [col setFill];
        [[NSBezierPath bezierPathWithRect:TD(px - sz * 0.5, py - sz * 0.5, sz, sz)] fill];
    }
}

// ============================================================================
// 10. 3D HEXAGONAL LASER CONTAINMENT FORCE-FIELD (HexShieldForcefieldCanvas)
// ============================================================================
static void drawHexShieldForcefield(double t, double triggerT, double cx, double cy, double baseR) {
    double activeU = eo(seg(t, triggerT, triggerT + 0.45));
    if (activeU <= 0.01) return;

    double pulse = 0.78 + 0.22 * sin(t * 9.0);

    // Subtle crimson aura
    NSGradient *aura = [[NSGradient alloc] initWithStartingColor:rgba(255, 95, 86, 0.28 * activeU * pulse)
                                                     endingColor:rgba(255, 95, 86, 0.0)];
    [aura drawFromCenter:P(cx, cy) radius:0.0 toCenter:P(cx, cy) radius:baseR * 2.4 options:0];

    for (int ring = 0; ring < 3; ring++) {
        double r = (baseR + ring * 26.0) * activeU;
        double rot = (ring % 2 == 0 ? 1.0 : -1.0) * t * (0.8 + ring * 0.35);
        NSBezierPath *hex = [NSBezierPath bezierPath];
        for (int k = 0; k < 6; k++) {
            double ang = rot + k * (M_PI / 3.0);
            double hx = cx + cos(ang) * r;
            double hy = cy + sin(ang) * r;
            if (k == 0) [hex moveToPoint:P(hx, hy)];
            else [hex lineToPoint:P(hx, hy)];
        }
        [hex closePath];
        double a = (0.72 - ring * 0.18) * pulse * activeU;
        [(ring == 0 ? rgba(255, 107, 94, a) : rgba(251, 179, 3, a)) setStroke];
        [hex setLineWidth:(ring == 0 ? 3.8 : 2.2)];
        [hex stroke];
    }
}

// ============================================================================
// 11. EXACT SHIPPED SAYWHEN HUD PILL
// ============================================================================
static void drawShippedHudPill(NSImage *logo, CGFloat cx, CGFloat cy, double scale,
                               int mode, NSString *label, NSString *sublabel, double rms, double countdownPct) {
    // mode: 0=ARMED, 1=LISTENING, 2=PREVIEW, 3=BLOCKED
    NSColor *accent = (mode == 0) ? C_ARMED :
                      (mode == 1) ? C_AMBER :
                      (mode == 2) ? C_AMBER_SOFT : C_DANGER;
    double rA, gA, bA;
    if (mode == 0) { rA = 184; gA = 204; bA = 228; }
    else if (mode == 1) { rA = 251; gA = 179; bA = 3; }
    else if (mode == 2) { rA = 255; gA = 201; bA = 77; }
    else { rA = 255; gA = 107; bA = 94; }

    NSFont *lf = fontSans(25, 600);
    NSFont *sf = fontMono(18, 500);
    CGSize lsz = measureText(label, lf, -0.3);
    CGSize ssz = sublabel.length > 0 ? measureText(sublabel, sf, 0.0) : CGSizeZero;

    CGFloat waveW = (mode == 1) ? 58.0 : 0.0;
    CGFloat divW = (sublabel.length > 0) ? (30.0 + ssz.width) : 0.0;
    CGFloat pw = (24.0 + 38.0 + 16.0 + 14.0 + 14.0 + lsz.width + waveW + divW + 30.0) * scale;
    CGFloat ph = 78.0 * scale;
    CGFloat px = cx - pw * 0.5;
    CGFloat py = cy - ph * 0.5;

    // Ambient accent glow shadow + dark plum capsule
    drawShadowRoundedBox(px, py, pw, ph, ph * 0.5, rgba(36, 7, 30, 0.96),
                         rgba(rA, gA, bA, 0.52), 2.0,
                         rgba(14, 2, 12, 0.56), 44.0, 18.0);

    // Inner specular highlight hairline on top
    drawRoundedBox(px + 2, py + 2, pw - 4, ph - 4, ph * 0.5 - 2, nil, rgba(255, 255, 255, 0.10), 1.0);

    CGFloat curX = px + 20.0 * scale;
    // Mini logo
    if (logo) {
        drawCroppedImageCard(logo, curX, cy - 19.0 * scale, 38.0 * scale, 38.0 * scale, 10.0 * scale,
                             rgba(255, 255, 255, 0.16), 1.0, NO);
    }
    curX += 54.0 * scale;

    // Glowing status dot
    NSGradient *dotGlow = [[NSGradient alloc] initWithStartingColor:rgba(rA, gA, bA, 0.45)
                                                        endingColor:rgba(rA, gA, bA, 0.0)];
    [dotGlow drawFromCenter:P(curX + 6 * scale, cy) radius:0.0
                   toCenter:P(curX + 6 * scale, cy) radius:18.0 * scale options:0];
    drawCircle(curX + 6 * scale, cy, 6.5 * scale, accent, nil, 0);
    curX += 24.0 * scale;

    // State label
    drawText(label, curX, cy - lsz.height * 0.5 - 1, lf, C_IVORY, -0.3);
    curX += lsz.width + 14.0 * scale;

    // Live 6-bar equalizer when LISTENING
    if (mode == 1) {
        const double mults[6] = { 0.45, 0.85, 1.0, 0.72, 0.94, 0.55 };
        for (int b = 0; b < 6; b++) {
            double bh = fmax(7.0, 28.0 * mults[b] * (0.45 + 0.65 * rms)) * scale;
            drawRoundedBox(curX + b * 8.0 * scale, cy - bh * 0.5, 4.5 * scale, bh, 2.2 * scale, C_AMBER, nil, 0);
        }
        curX += 56.0 * scale;
    }

    // Divider & sublabel
    if (sublabel.length > 0) {
        drawRoundedBox(curX, cy - 12 * scale, 1.5, 24 * scale, 0.5, rgba(223, 229, 236, 0.20), nil, 0);
        curX += 16.0 * scale;
        drawText(sublabel, curX, cy - ssz.height * 0.5 - 1, sf, C_MUTED_DK, 0.0);
    }

    // Countdown progress bar along bottom edge when PREVIEW
    if (mode == 2 && countdownPct > 0.0) {
        drawRoundedBox(px + 28, py + ph - 6, (pw - 56) * clamp01(countdownPct), 3.5, 2.0, C_AMBER, nil, 0);
    }
}

// ============================================================================
// 12. HIGH-SPEED VERTICAL SLOT REEL (Beat 11)
// ============================================================================
static double reelPosition(double tau, int lastIdx) {
    const double RV = 5.40;
    const double RT0 = 0.14;
    const double RT1 = RT0 + 0.16;
    double RP1 = 0.5 * RV * (RT1 - RT0);
    double RT2 = RT1 + ((double)(lastIdx - 1) - RP1) / RV;
    const double ROV = 0.22;
    double RD = (2.0 * (1.0 + ROV)) / RV;
    double RT3 = RT2 + RD;
    double RT4 = RT3 + 0.28;

    if (tau < RT0) return 0.0;
    if (tau < RT1) return 0.5 * (RV / (RT1 - RT0)) * pow(tau - RT0, 2.0);
    if (tau < RT2) return RP1 + RV * (tau - RT1);
    if (tau < RT3) {
        double x = tau - RT2;
        return (double)(lastIdx - 1) + RV * x - 0.5 * (RV / RD) * x * x;
    }
    return lerpd((double)lastIdx + ROV, (double)lastIdx, eio(seg(tau, RT3, RT4)));
}

static void drawVerticalSlotReel(double tau, CGFloat centerY, BOOL lightPlate,
                                 NSArray<NSString *> *items, double *outVelocity, double *outLandPop) {
    int n = (int)items.count;
    int lastIdx = n - 1;
    double rP = reelPosition(tau, lastIdx);
    double rNext = reelPosition(tau + DT, lastIdx);
    double vel = fabs(rNext - rP) * 180.0;
    if (outVelocity) *outVelocity = vel;

    // Landing time calculation matching reelPosition
    const double RV = 5.40;
    const double RT0 = 0.14;
    const double RT1 = RT0 + 0.16;
    double RP1 = 0.5 * RV * (RT1 - RT0);
    double RT2 = RT1 + ((double)(lastIdx - 1) - RP1) / RV;
    double RD = (2.0 * 1.22) / RV;
    double RT4 = RT2 + RD + 0.28;
    double landPop = seg(tau, RT4, RT4 + 0.12) * (1.0 - seg(tau, RT4 + 0.12, RT4 + 0.42));
    if (outLandPop) *outLandPop = landPop;

    const CGFloat REH = 175.0;

    // Hairline top & bottom slot gate rules
    NSColor *ruleCol = lightPlate ? rgba(42, 8, 35, 0.16) : rgba(223, 229, 236, 0.18);
    drawRoundedBox(160, centerY - REH * 0.5 - 16, W - 320, 2.0, 1.0, ruleCol, nil, 0);
    drawRoundedBox(160, centerY + REH * 0.5 + 16, W - 320, 2.0, 1.0, ruleCol, nil, 0);

    for (int i = 0; i < n; i++) {
        double d = (double)i - rP;
        if (fabs(d) > 2.5) continue;
        double ad = fabs(d);
        double op = clamp01(1.0 - 0.65 * fmin(ad, 1.45));
        BOOL isLast = (i == lastIdx);
        double scale = (1.0 - 0.12 * fmin(ad, 1.6)) * (isLast ? (1.0 + 0.055 * landPop) : 1.0);

        NSString *txt = items[(NSUInteger)i];
        CGFloat fSize = (txt.length > 18 ? 64.0 : 72.0) * scale;
        NSFont *f = fontSans(fSize, 800);
        NSColor *col = isLast
            ? rgba(lightPlate ? 212 : 251, lightPlate ? 128 : 179, 3, op)
            : (lightPlate ? rgba(42, 8, 35, op * 0.85) : rgba(246, 238, 244, op * 0.90));

        CGSize sz = measureText(txt, f, -2.2);
        CGFloat y = centerY + d * REH - sz.height * 0.5;
        drawCenteredText(txt, y, f, col, -2.2);
    }
}

// ============================================================================
// 13. FIBONACCI 3D GOLDEN VORTEX FINALE (FibonacciVortexCanvas)
// ============================================================================
static void drawFibonacciVortex(double t, double rms, double cx, double cy) {
    const int N = 240;
    const double goldenAngle = M_PI * (3.0 - sqrt(5.0));
    double settle = eo(seg(t, 0.0, 1.20));

    for (int i = 0; i < N; i++) {
        double frac = (double)i / (double)N;
        double baseR = lerpd(780.0, 260.0 + 360.0 * sqrt(frac), settle);
        double theta = i * goldenAngle + t * (0.45 - frac * 0.22);
        double z = sin(frac * M_PI * 4.0 + t * 2.0) * (140.0 * (1.0 - settle * 0.5));

        double x = cos(theta) * baseR;
        double y = sin(theta) * baseR * 0.88;
        Proj3D p = project3D(x, y, z, 0.25 * (1.0 - settle), 0, 0, cx, cy, 1100.0);
        double rad = (1.8 + (1.0 - frac) * 3.4 + rms * 2.2) * p.scale;
        double a = clampd(0.20 + (1.0 - frac) * 0.65, 0.15, 0.85);
        NSColor *col = (i % 3 == 0) ? rgba(251, 179, 3, a) : rgba(223, 229, 236, a * 0.72);
        drawCircle(p.sx, p.sy, rad, col, nil, 0);
    }
}

// ============================================================================
// SHARED UI: SUBTITLE CAPSULE & EDITORIAL FINALE
// ============================================================================
static void drawSubtitlePill(NSString *line1, NSString *line2, double alpha) {
    if (alpha <= 0.01) return;
    NSFont *cf = fontSans(29, 600);
    NSString *full = line2.length > 0 ? [NSString stringWithFormat:@"%@ %@", line1, line2] : line1;
    CGSize fullSz = measureText(full, cf, -0.3);
    CGSize p1Sz = measureText([NSString stringWithFormat:@"%@ ", line1], cf, -0.3);

    CGFloat pw = fmin(W - 88, fullSz.width + 64);
    CGFloat ph = 66;
    CGFloat px = (W - pw) * 0.5;
    CGFloat py = 1604;

    drawShadowRoundedBox(px, py, pw, ph, 33, rgba(22, 4, 20, 0.94 * alpha),
                         rgba(255, 255, 255, 0.20 * alpha), 1.5,
                         rgba(0, 0, 0, 0.45 * alpha), 22.0, 10.0);
    CGFloat tx = (W - fullSz.width) * 0.5;
    CGFloat ty = py + (ph - fullSz.height) * 0.5 - 2;
    drawText(line1, tx, ty, cf, rgba(246, 238, 244, alpha), -0.3);
    if (line2.length > 0) {
        drawText(line2, tx + p1Sz.width, ty, cf, rgba(251, 179, 3, alpha), -0.3);
    }
}

static void drawHandUnderline(CGFloat x, CGFloat y, CGFloat w, double frac, NSColor *col) {
    if (frac <= 0.01) return;
    NSBezierPath *ul = [NSBezierPath bezierPath];
    [ul moveToPoint:P(x, y)];
    [ul curveToPoint:P(x + w * clamp01(frac), y - 2.0)
       controlPoint1:P(x + w * 0.35 * clamp01(frac), y + 4.5)
       controlPoint2:P(x + w * 0.72 * clamp01(frac), y - 3.5)];
    [col setStroke];
    [ul setLineCapStyle:NSLineCapStyleRound];
    [ul setLineWidth:7.0];
    [ul stroke];
}

static void drawStudioFinale(int frame, double localT, NSImage *logo, NSImage *appCard,
                             NSString *tagline, NSString *subBadge) {
    drawPlateBackground(NO, 1.0, 0.0);
    drawHorizonGrid3D(frame, NO, 0.9, 0.0);
    drawFibonacciVortex(localT, frameRms(frame), CX, 780);

    double logoU = eo(seg(localT, 0.05, 0.48));
    CGFloat lw = 176.0 * lerpd(0.88, 1.0, logoU);
    drawShadowRoundedBox((W - lw) * 0.5, 430 - (lw - 176.0) * 0.5, lw, lw, 40, rgba(34, 6, 28, logoU),
                         rgba(251, 179, 3, 0.50 * logoU), 2.2, rgba(251, 179, 3, 0.45 * logoU), 42.0, 0.0);
    drawCroppedImageCard(logo, (W - lw) * 0.5, 430 - (lw - 176.0) * 0.5, lw, lw, 40, nil, logoU, NO);

    double wordU = eo(seg(localT, 0.18, 0.62));
    drawCenteredText(@"saywhen", 632 + (1.0 - wordU) * 18.0, fontSans(112, 800), rgba(223, 229, 236, wordU), -4.2);

    double tagU = eo(seg(localT, 0.42, 0.85));
    drawCenteredText(tagline, 770 + (1.0 - tagU) * 16.0, fontSerif(74, YES), rgba(251, 179, 3, tagU), -1.5);

    if (appCard) {
        double cardU = eo(seg(localT, 0.52, 0.98));
        drawCroppedImageCard(appCard, 128, 905 + (1.0 - cardU) * 26.0, W - 256, 568, 22,
                             rgba(255, 255, 255, 0.20 * cardU), cardU, YES);
    }

    double ctaU = seg(localT, 0.55, 0.95);
    if (ctaU > 0.01) {
        double sc = lerpd(0.75, 1.0, bo(ctaU));
        CGFloat cw = 640.0 * sc;
        CGFloat ch = 74.0 * sc;
        CGFloat cx0 = (W - cw) * 0.5;
        CGFloat cy0 = 1522 + (74.0 - ch) * 0.5;
        drawShadowRoundedBox(cx0, cy0, cw, ch, 37, C_AMBER, nil, 0, rgba(251, 179, 3, 0.34), 32.0, 12.0);
        NSString *ctaTxt = @"Hands-free voice for developers";
        NSFont *cf = fontSans(25, 700);
        CGSize csz = measureText(ctaTxt, cf, -0.4);
        CGFloat tx = (W - (csz.width + 34.0)) * 0.5;
        CGFloat ty = cy0 + (ch - csz.height) * 0.5 - 1;
        drawText(ctaTxt, tx, ty, cf, C_PLUM_INK, -0.4);

        // Vector right arrow icon
        CGFloat ax = tx + csz.width + 14.0;
        CGFloat ay = cy0 + ch * 0.5;
        NSBezierPath *arr = [NSBezierPath bezierPath];
        [arr moveToPoint:P(ax, ay)];
        [arr lineToPoint:P(ax + 18.0, ay)];
        [arr moveToPoint:P(ax + 11.0, ay - 7.0)];
        [arr lineToPoint:P(ax + 18.5, ay)];
        [arr lineToPoint:P(ax + 11.0, ay + 7.0)];
        [C_PLUM_INK setStroke];
        [arr setLineWidth:3.0];
        [arr setLineCapStyle:NSLineCapStyleRound];
        [arr setLineJoinStyle:NSLineJoinStyleRound];
        [arr stroke];

        drawCenteredText(subBadge, 1618, fontMono(16, 600), C_MUTED_DK, 2.0);
    }
}

// ============================================================================
// REEL HERO: 15.0s (450 frames) FLAGSHIP ALL-IN-ONE AESTHETIC CUT
// ============================================================================
static void renderReelHeroFrame(int frame, NSBitmapImageRep *rep, NSImage *logo, NSImage *uiReady, NSImage *uiListening) {
    double t = frame / FPS;
    double rms = frameRms(frame);
    int blurX = 0, blurY = 0;

    if (frame < 86) {
        // BEAT 1 (f=0..70) + HALFTONE IRIS TRANSITION into Beat 2 (f=70..86)
        drawPlateBackground(NO, 1.0, 0.0);
        drawHorizonGrid3D(frame, NO, 1.0, 0.0);

        double shatterStart = 1.75;
        double boom = eo(seg(t, shatterStart, shatterStart + 0.62));
        double fade = 1.0 - seg(t, shatterStart + 0.32, shatterStart + 0.62);

        if (boom < 0.85) {
            drawText(@"THE OLD WAY · PUSH-TO-TALK FRICTION", 76, 205, fontMono(18, 700), C_AMBER, 2.2);
            drawText(@"Why does voice", 72, 246, fontSans(76, 800), C_IVORY, -2.6);
            drawText(@"still need a ", 72, 336, fontSans(76, 800), C_IVORY, -2.6);
            CGSize prefixW = measureText(@"still need a ", fontSans(76, 800), -2.6);
            drawText(@"chord?", 72 + prefixW.width, 336, fontSans(76, 800), C_AMBER, -2.6);

            double cardFade = (boom > 0.02) ? clamp01(1.0 - boom * 2.8) : 1.0;
            drawTiltedProblemCard(530, 620, 840, 232, -2.0, @"ATTEMPT 01", @"Hold Fn + Space",
                                  @"Finger cramps mid-sentence", @"x dropped early!",
                                  cardFade * eo(seg(t, 0.0, 0.22)), ease(t, 0.32, 0.64), 101u);
            drawTiltedProblemCard(550, 915, 840, 232, 1.6, @"ATTEMPT 02", @"Press OPT + R to toggle",
                                  @"Recorded 40s in wrong app", @"x wrong window focused",
                                  cardFade * eo(seg(t, 0.14, 0.38)), ease(t, 0.50, 0.82), 202u);
            drawTiltedProblemCard(528, 1210, 840, 232, -1.5, @"ATTEMPT 03", @"Paste, then press Enter",
                                  @"Still reaching for the keyboard", @"x still using hands",
                                  cardFade * eo(seg(t, 0.28, 0.52)), ease(t, 0.68, 1.00), 303u);
        }

        drawShatteringKeycaps3D(boom, fade, CX, 910);

        if (frame >= 70) {
            double irisP = eo(seg(t, 70.0 / 30.0, 85.0 / 30.0));
            double irisR = lerpd(30.0, 1220.0, irisP);
            double b2T = (frame - 70) / FPS;
            double lookA = eo(seg(b2T, 0.05, 0.35));
            [NSGraphicsContext saveGraphicsState];
            beginHalftoneIrisClip(irisR);
            drawPlateBackground(YES, 1.0, 0.0);
            drawHorizonGrid3D(frame, YES, 0.9, 0.0);
            drawCenteredText(@"Look.", 228 + (1.0 - lookA) * 20.0, fontSerif(126, YES), rgba(42, 8, 35, lookA), -2.4);
            drawParametricRibbon3D(frame, b2T, 0.0, 0.0, CX, 960, 1.0);
            drawShippedHudPill(logo, CX, 1410, 1.06, 0, @"Ready · facing screen", @"Yaw -1.2 deg · Pitch +0.8 deg", rms, 0.0);
            [NSGraphicsContext restoreGraphicsState];
            drawHalftoneIrisRing(irisR, 150.0, 1.0 - seg(irisR, 960.0, 1220.0));
        }
    } else if (frame < 158) {
        // BEAT 2 (f=70..158): Warm Ivory + 3D Parametric Ribbon + "Look. Talk. Sent." + Shipped HUD Pill
        double b2T = (frame - 70) / FPS;
        double m01 = eio(seg(b2T, 0.75, 1.25));
        double m12 = eio(seg(b2T, 1.65, 2.15));

        drawPlateBackground(YES, 1.0, 0.0);
        drawHorizonGrid3D(frame, YES, 0.95, 0.0);
        drawParametricRibbon3D(frame, b2T, m01, m12, CX, 960, 1.0);

        double lookA = eo(seg(b2T, 0.05, 0.35));
        double talkA = eo(seg(b2T, 0.75, 1.05));
        double sentA = eo(seg(b2T, 1.50, 1.85));

        drawCenteredText(@"Look.", 228 + (1.0 - lookA) * 20.0, fontSerif(126, YES), rgba(42, 8, 35, lookA), -2.4);
        drawCenteredText(@"Talk.", 358 + (1.0 - talkA) * 20.0, fontSerif(126, YES), rgba(42, 8, 35, talkA), -2.4);
        drawCenteredText(@"Sent.", 488 + (1.0 - sentA) * 20.0, fontSerif(126, YES), rgba(212, 134, 0, sentA), -2.4);

        CGSize ssz = measureText(@"Sent.", fontSerif(126, YES), -2.4);
        drawHandUnderline((W - ssz.width) * 0.5, 622, ssz.width, ease(b2T, 1.70, 2.15), C_AMBER);

        int hudMode = (b2T < 0.85) ? 0 : (b2T < 1.80) ? 1 : 2;
        NSString *hudLabel = (hudMode == 0) ? @"Ready · facing screen" :
                             (hudMode == 1) ? @"Listening live" : @"Sending in 0.8s";
        NSString *hudSub = (hudMode == 0) ? @"Yaw -1.2 deg · Pitch +0.8 deg" :
                           (hudMode == 1) ? @"500 ms pre-roll" : @"Esc or shake to cancel";
        double cdPct = (hudMode == 2) ? clamp01(1.0 - (b2T - 1.80) / 1.10) : 0.0;
        drawShippedHudPill(logo, CX, 1410, 1.06, hudMode, hudLabel, hudSub, rms, cdPct);

        if (frame >= 152) {
            blurX = 16;
        }
    } else if (frame < 236) {
        // BEAT 3 (f=158..236): 3D 478-Point Biometric Face Mesh + Gaze Cone + UI Window
        double b3T = (frame - 158) / FPS;
        double lockU = eio(seg(b3T, 0.05, 0.72));
        if (frame < 164) blurX = (int)((164 - frame) * 4);

        drawPlateBackground(NO, 1.0, 0.0);
        drawHorizonGrid3D(frame, NO, 0.9, 120.0);

        drawText(@"01 · WEBCAM ATTENTION GATE", 72, 200, fontMono(18, 700), C_ARMED, 2.2);
        drawText(@"Attention replaces", 72, 238, fontSans(68, 800), C_IVORY, -2.4);
        drawText(@"the push-to-talk key.", 72, 318, fontSans(68, 800), C_AMBER, -2.4);

        // 3D Face mesh with downward volumetric golden gaze cone onto UI card
        drawBiometricFaceMesh3D(b3T, rms, lockU, 0.0, CX, 615, 0.84, 1080.0, 410.0);

        // Real ui-dark-ready.png window
        drawCroppedImageCard(uiReady, 72, 920, W - 144, 480, 22, rgba(251, 179, 3, 0.36), 1.0, YES);

        // Foreground Crisp White Telemetry Card (exact style from _b3.png)
        drawShadowRoundedBox(108, 1188, W - 216, 336, 26, C_WHITE, rgba(34, 6, 28, 0.10), 1.5,
                             rgba(0, 0, 0, 0.45), 36.0, 16.0);
        drawText(@"ON-DEVICE FACE TRACKING · 15 FPS", 142, 1222, fontMono(16, 700), C_MUTED_LT, 1.8);
        drawRoundedBox(W - 434, 1212, 290, 36, 18, C_AMBER, nil, 0);
        drawText(@"FACING SCREEN · READY", W - 416, 1221, fontMono(14, 700), C_PLUM_INK, 0.8);
        drawText(@"See you look. Keep word one.", 142, 1266, fontSans(34, 800), C_PLUM_INK, -1.0);

        struct { NSString *k; NSString *v; NSString *s; } tiles[3] = {
            { @"Head Yaw", @"-1.4 deg", @"Facing display" },
            { @"Lip Delta", @"0.042", @"Motion active" },
            { @"Pre-Roll", @"500 ms", @"First word kept" }
        };
        for (int i = 0; i < 3; i++) {
            CGFloat tx = 142 + i * 268;
            drawRoundedBox(tx, 1340, 248, 150, 16, C_IVORY, rgba(34, 6, 28, 0.08), 1.2);
            drawText(tiles[i].k, tx + 20, 1360, fontSans(18, 600), C_MUTED_LT, 0.0);
            drawText(tiles[i].v, tx + 20, 1396, fontMono(30, 800), C_PLUM_INK, -0.8);
            drawText(tiles[i].s, tx + 20, 1448, fontSans(17, 500), C_MUTED_LT, 0.0);
        }
    } else if (frame < 350) {
        // BEAT 4 (f=236..350): 3D Acoustic Terrain + Wet-Ink Cleanup -> LightSpeed Warp into Ghostty
        double b4T = (frame - 236) / FPS;
        if (frame < 296) {
            drawPlateBackground(YES, 1.0, 0.0);
            drawAcousticTerrain3D(frame, 1490.0, YES, 1.0);

            drawText(@"02 · DETERMINISTIC CLEANUP · CLEANUP.RS", 72, 200, fontMono(17, 700), C_MUTED_LT, 2.0);
            drawText(@"Speak like a person.", 72, 238, fontSans(66, 800), C_PLUM_INK, -2.4);
            NSFont *hf = fontSans(66, 800);
            CGSize hsz = measureText(@"Write like a developer.", hf, -2.4);
            drawRoundedBox(62, 320, hsz.width + 32, 86, 14, rgba(251, 179, 3, 0.30), nil, 0);
            drawText(@"Write like a developer.", 78, 322, hf, C_PLUM_INK, -2.4);

            // Wet-Ink Prompt Card
            drawShadowRoundedBox(68, 456, W - 136, 360, 28, C_WHITE, rgba(251, 179, 3, 0.85), 2.5,
                                 rgba(34, 6, 28, 0.14), 34.0, 14.0);
            drawRoundedBox(102, 488, 378, 38, 19, C_AMBER, nil, 0);
            drawText(@"LIVE AST COMPILER · 0 MS CLOUD", 120, 498, fontMono(14, 700), C_PLUM_INK, 0.9);
            drawText(@"16 kHz Silero VAD", W - 306, 497, fontMono(15, 600), C_MUTED_LT, 0.0);

            NSArray<NSString *> *spoken = @[
                @"um git checkout dash b",
                @"feat slash voice gate",
                @"and cargo test send it"
            ];
            drawWetInkLines(spoken, b4T, 0.04, 42.0, 0.40, 102, 560, 62, fontMono(34, 700),
                            42, 8, 35, 212, 134, 0, YES);

            // Token compilation pills below card
            struct { NSString *from; NSString *to; double at; } toks[3] = {
                { @"dash b", @"-b", 0.32 },
                { @"slash", @"/", 0.58 },
                { @"and ... send it", @"&& [Enter]", 0.84 }
            };
            for (int i = 0; i < 3; i++) {
                double u = eo(seg(b4T, toks[i].at, toks[i].at + 0.25));
                if (u <= 0.01) continue;
                CGFloat tx = 68 + i * 320;
                drawShadowRoundedBox(tx, 850 + (1.0 - u) * 16.0, 300, 118, 20, C_WHITE, C_AMBER, 1.8,
                                     rgba(34, 6, 28, 0.12), 20.0, 8.0);
                drawText(toks[i].from, tx + 24, 872, fontMono(19, 500), C_MUTED_LT, 0.0);
                drawText(@"->", tx + 24, 908, fontMono(22, 700), C_AMBER_DARK, 0.0);
                drawText(toks[i].to, tx + 64, 906, fontMono(26, 800), C_PLUM_INK, -0.4);
            }

            if (frame >= 288) blurY = 14;
        } else {
            // LightSpeed Warp + Ghostty Terminal Execution (f=296..350)
            double b4bT = (frame - 296) / FPS;
            drawPlateBackground(NO, 1.0, 0.0);
            drawHorizonGrid3D(frame, NO, 0.85, 0.0);

            drawText(@"03 · FOCUSED TERMINAL INJECTION", 72, 200, fontMono(18, 700), C_EMERALD, 2.2);
            drawText(@"Typed. Compiled.", 72, 240, fontSans(70, 800), C_IVORY, -2.4);
            drawText(@"Enter pressed.", 72, 322, fontSans(70, 800), C_EMERALD, -2.4);

            // Ghostty Terminal Window
            BOOL testsOk = (b4bT >= 0.75);
            drawShadowRoundedBox(68, 456, W - 136, 980, 28, rgba(11, 5, 13, 0.98),
                                 testsOk ? C_EMERALD : rgba(255, 255, 255, 0.20), 2.5,
                                 rgba(0, 0, 0, 0.55), 42.0, 18.0);
            drawCircle(114, 504, 8.5, rgba(255, 95, 86, 1.0), nil, 0);
            drawCircle(142, 504, 8.5, rgba(255, 189, 46, 1.0), nil, 0);
            drawCircle(170, 504, 8.5, rgba(39, 201, 63, 1.0), nil, 0);
            drawText(@"ghostty -- ~/Projects/saywhen", 210, 491, fontMono(20, 600), C_MUTED_DK, 0.0);

            drawText(@"$ git checkout -b feat/voice-gate \\", 106, 578, fontMono(30, 700), C_AMBER, -0.5);
            drawText(@"    && cargo test --release", 106, 624, fontMono(30, 700), C_AMBER, -0.5);

            double outU = eo(seg(b4bT, 0.20, 0.75));
            if (outU > 0.01) {
                drawText(@"Switched to a new branch 'feat/voice-gate'", 106, 708, fontMono(22, 500), rgba(130, 232, 255, outU), 0.0);
                drawText(@"   Compiling saywhen v0.2.1 (src-tauri)", 106, 764, fontMono(22, 500), rgba(223, 229, 236, outU), 0.0);
                drawText(@"test turn_taking::adaptive_pause ... ok", 106, 840, fontMono(21, 500), rgba(199, 184, 196, outU), 0.0);
                drawText(@"test cleanup::developer_cli_pass ... ok", 106, 886, fontMono(21, 500), rgba(199, 184, 196, outU), 0.0);
                drawText(@"test inject::allowlist_gate ... ok", 106, 932, fontMono(21, 500), rgba(199, 184, 196, outU), 0.0);
                drawText(@"test wake_word::alias_strip ... ok", 106, 978, fontMono(21, 500), rgba(199, 184, 196, outU), 0.0);
            }

            if (testsOk) {
                double okU = bo(seg(b4bT, 0.75, 1.05));
                drawRoundedBox(106, 1080, W - 212, 130, 20, rgba(61, 220, 151, 0.16), C_EMERALD, 2.2);
                drawText(@"OK  test result: ok. 47 passed; 0 failed", 138, 1116, fontMono(28, 700), C_EMERALD, -0.5);
                drawText([NSString stringWithFormat:@"Allowlisted terminal · %.2fx", okU], 138, 1162, fontMono(18, 500), C_SILVER, 0.0);
            }

            drawLightSpeedWarp(b4bT, 0.0, CX, CY);
            drawPixelBurst(b4bT, 0.04, CX, 560);
        }
    } else if (frame < 398) {
        // BEAT 5A (f=350..398): High-Speed Vertical Slot Reel on Deep Plum
        double b5T = (frame - 350) / FPS;
        drawPlateBackground(NO, 1.0, 0.0);
        drawHorizonGrid3D(frame, NO, 0.95, 0.0);

        drawCenteredText(@"LOCAL-FIRST ARCHITECTURE", 280, fontMono(18, 700), C_AMBER, 2.4);

        NSArray<NSString *> *slotItems = @[
            @"MediaPipe 15 FPS.",
            @"Silero VAD v5.",
            @"Whisper Metal GPU.",
            @"Zero Telemetry.",
            @"Cursor & Claude.",
            @"Runs On Your Computer."
        ];
        double vel = 0.0, landPop = 0.0;
        drawVerticalSlotReel(b5T, CY - 50, NO, slotItems, &vel, &landPop);
        if (vel > 12.0) blurY = (int)fmin(14.0, vel * 0.14);

        if (landPop > 0.0 || b5T > 0.90) {
            drawCenteredText(@"Camera in RAM · Audio discarded · 0 bytes sent", 1410, fontMono(20, 600), C_MUTED_DK, 0.6);
        }
    } else {
        // BEAT 5B (f=398..450): Fibonacci Golden Vortex Finale
        double finT = (frame - 398) / FPS;
        drawStudioFinale(frame, finT, logo, uiListening, @"Look. Talk. Sent.", @"FREE & OPEN SOURCE · MAC + WINDOWS");
    }

    // Apply 180-degree shutter blur if moving fast
    if (blurX > 1 || blurY > 1) {
        applyDirectionalMotionBlur(rep, blurX, blurY);
    }

    // Editorial Subtitles (in Instagram safe zone y=1604, omitted during Look/Talk/Sent & Finale)
    if (frame >= 5 && frame <= 70) {
        drawSubtitlePill(@"Voice input.", @"Without a hotkey.", ease(frame, 4, 10) * (1.0 - ease(frame, 64, 70)));
    } else if (frame >= 158 && frame <= 216) {
        drawSubtitlePill(@"It can see you", @"looking at the screen.", ease(frame, 158, 164) * (1.0 - ease(frame, 210, 216)));
    } else if (frame >= 238 && frame <= 344) {
        drawSubtitlePill(@"You speak like a person.", @"It writes like a developer.", ease(frame, 238, 244) * (1.0 - ease(frame, 338, 344)));
    } else if (frame >= 354 && frame <= 402) {
        drawSubtitlePill(@"Nothing leaves", @"this computer.", ease(frame, 354, 360) * (1.0 - ease(frame, 396, 402)));
    }
}

// ============================================================================
// REEL 1: 11.8s (354 frames) "Look. Talk. Sent."
// VO: 01(5..70), 03(74..124), 04(126..207), 06(209..276), 02(278..350)
// ============================================================================
static void renderReel1Frame(int frame, NSBitmapImageRep *rep, NSImage *logo, NSImage *uiReady) {
    double t = frame / FPS;
    double rms = frameRms(frame);
    int blurX = 0, blurY = 0;

    if (frame < 72) {
        // Scene 1: Hotkey Tax & 3D Shatter (Dark Plum)
        drawPlateBackground(NO, 1.0, 0.0);
        drawHorizonGrid3D(frame, NO, 1.0, 0.0);

        drawText(@"FOR CODING AGENTS & TERMINALS", 72, 202, fontMono(18, 700), C_AMBER, 2.2);
        drawText(@"Voice input.", 68, 246, fontSerif(110, YES), C_IVORY, -2.0);
        drawText(@"Without a hotkey.", 68, 354, fontSerif(110, YES), C_AMBER, -2.0);

        double boom = eo(seg(t, 1.80, 2.38));
        double fade = 1.0 - seg(t, 2.12, 2.38);
        double cFade = (boom > 0.02) ? clamp01(1.0 - boom * 3.0) : 1.0;

        drawTiltedProblemCard(535, 650, 850, 236, -2.2, @"THE OLD WAY · CARD 01", @"Hold Fn + Space",
                              @"Cut off mid-sentence every time", @"x dropped early!",
                              cFade, ease(t, 0.45, 0.85), 111u);
        drawTiltedProblemCard(548, 945, 850, 236, 1.6, @"THE OLD WAY · CARD 02", @"Press OPT + R",
                              @"Recorded into Slack instead of Ghostty", @"x wrong window!",
                              cFade * eo(seg(t, 0.18, 0.44)), ease(t, 0.72, 1.12), 222u);

        drawCroppedImageCard(uiReady, 72, 1115, W - 144, 415, 22, rgba(251, 179, 3, 0.34 * cFade), cFade * 0.92, YES);
        drawShatteringKeycaps3D(boom, fade, CX, 820);
    } else if (frame < 126) {
        // Scene 2: Halftone Iris -> Warm Ivory + 3D Parametric Ribbon + Face Mesh Gaze Cone
        double s2T = (frame - 72) / FPS;
        double irisR = lerpd(40.0, 1280.0, eo(seg(s2T, 0.0, 0.45)));

        drawPlateBackground(NO, 0.6, 0.0);
        [NSGraphicsContext saveGraphicsState];
        beginHalftoneIrisClip(irisR);

        drawPlateBackground(YES, 1.0, 0.0);
        drawHorizonGrid3D(frame, YES, 0.9, 0.0);
        drawParametricRibbon3D(frame, s2T, eio(seg(s2T, 0.85, 1.45)), 0.0, CX, 920, 0.96);

        drawCenteredText(@"01 · WEBCAM ATTENTION GATE", 206, fontMono(18, 700), C_MUTED_LT, 2.2);
        drawCenteredText(@"Look at your screen.", 246, fontSerif(94, YES), C_PLUM_INK, -1.8);
        drawShippedHudPill(logo, CX, 1380, 1.08, 0, @"Ready · facing screen", @"15 FPS local · 0 frames saved", rms, 0.0);

        [NSGraphicsContext restoreGraphicsState];
        if (irisR < 1260.0) {
            drawHalftoneIrisRing(irisR, 135.0, 1.0 - seg(irisR, 980.0, 1260.0));
        }
    } else if (frame < 208) {
        // Scene 3: 3D Biometric Face Mesh + Acoustic Terrain + Live Capture (Dark Plum)
        double s3T = (frame - 126) / FPS;
        double lockU = eio(seg(s3T, 0.0, 0.45));
        if (frame < 132) blurX = (132 - frame) * 4;

        drawPlateBackground(NO, 1.0, 0.0);
        drawHorizonGrid3D(frame, NO, 0.85, 140.0);

        drawText(@"02 · JUST TALK · 500 MS PRE-ROLL RING", 72, 200, fontMono(18, 700), C_AMBER, 2.0);
        drawText(@"No button to hold.", 72, 238, fontSans(74, 800), C_IVORY, -2.5);

        drawBiometricFaceMesh3D(s3T, rms, lockU, 0.0, CX, 550, 0.78, 950.0, 400.0);

        // Elevated White Live Transcript Card
        drawShadowRoundedBox(68, 860, W - 136, 330, 26, C_WHITE, C_AMBER, 2.4, rgba(0, 0, 0, 0.48), 38.0, 18.0);
        drawText(@"SAYWHEN · LISTENING LIVE", 104, 896, fontMono(16, 700), C_AMBER_DARK, 1.6);
        drawText(@"First syllable captured", W - 344, 896, fontMono(15, 600), C_MUTED_LT, 0.0);

        NSArray<NSString *> *cap = @[ @"\"cargo test dash dash release\"" ];
        drawWetInkLines(cap, s3T, 0.08, 22.0, 0.40, 104, 965, 58, fontMono(36, 800), 42, 8, 35, 212, 134, 0, YES);
        drawText(@"Silero VAD v5 + circular pre-roll ring buffer", 104, 1098, fontSans(22, 500), C_MUTED_LT, 0.0);

        drawShippedHudPill(logo, CX, 1365, 1.08, 1, @"Listening live", @"0.8s pause -> Send", rms, 0.0);
    } else if (frame < 278) {
        // Scene 4: Warm Ivory Preview -> LightSpeed Warp into Ghostty Terminal
        double s4T = (frame - 208) / FPS;
        if (s4T < 0.95) {
            drawPlateBackground(YES, 1.0, 0.0);
            drawAcousticTerrain3D(frame, 1480.0, YES, 1.0);
            drawText(@"03 · CANCELABLE PREVIEW", 72, 205, fontMono(18, 700), C_MUTED_LT, 2.2);
            drawText(@"Types cleanly.", 72, 246, fontSerif(104, YES), C_PLUM_INK, -2.0);
            drawText(@"Presses Enter.", 72, 348, fontSerif(104, YES), C_AMBER_DARK, -2.0);

            drawShadowRoundedBox(68, 520, W - 136, 300, 28, C_WHITE, C_AMBER, 2.6, rgba(34, 6, 28, 0.16), 36.0, 16.0);
            drawText(@"AUTO-SENDING IN 0.8S · ESC TO CANCEL", 104, 558, fontMono(16, 700), C_AMBER_DARK, 1.5);
            drawText(@"cargo test --release", 104, 616, fontMono(42, 800), C_PLUM_INK, -0.8);
            double pct = clamp01(1.0 - s4T / 0.95);
            drawRoundedBox(104, 738, (W - 208) * fmax(0.08, pct), 12, 6, C_AMBER, nil, 0);
        } else {
            double termT = s4T - 0.95;
            drawPlateBackground(NO, 1.0, 0.0);
            drawHorizonGrid3D(frame, NO, 0.85, 0.0);
            drawText(@"03 · GHOSTTY TERMINAL", 72, 205, fontMono(18, 700), C_EMERALD, 2.2);
            drawText(@"Sent.", 72, 248, fontSerif(114, YES), C_EMERALD, -2.0);

            drawShadowRoundedBox(68, 430, W - 136, 980, 28, rgba(10, 5, 12, 0.98), C_EMERALD, 2.5,
                                 rgba(0, 0, 0, 0.55), 42.0, 18.0);
            drawCircle(114, 478, 8.5, rgba(255, 95, 86, 1.0), nil, 0);
            drawCircle(142, 478, 8.5, rgba(255, 189, 46, 1.0), nil, 0);
            drawCircle(170, 478, 8.5, rgba(39, 201, 63, 1.0), nil, 0);
            drawText(@"ghostty -- ~/Projects/saywhen", 210, 465, fontMono(20, 600), C_MUTED_DK, 0.0);

            drawText(@"$ cargo test --release", 106, 560, fontMono(36, 700), C_AMBER, -0.5);
            drawText(@"Compiling saywhen v0.2.1 (src-tauri)", 106, 650, fontMono(22, 500), C_SILVER, 0.0);
            drawText(@"Running unittests src/lib.rs", 106, 698, fontMono(22, 500), C_SILVER, 0.0);
            drawText(@"test turn_taking::adaptive_pause ... ok", 106, 770, fontMono(20, 500), C_MUTED_DK, 0.0);
            drawText(@"test cleanup::developer_cli_pass ... ok", 106, 818, fontMono(20, 500), C_MUTED_DK, 0.0);

            if (termT >= 0.45) {
                drawRoundedBox(106, 940, W - 212, 120, 20, rgba(61, 220, 151, 0.16), C_EMERALD, 2.2);
                drawText(@"OK  test result: ok. 47 passed; 0 failed", 136, 980, fontMono(28, 700), C_EMERALD, -0.4);
            }
            drawLightSpeedWarp(termT, 0.0, CX, CY);
            drawPixelBurst(termT, 0.04, CX, 560);
        }
    } else {
        // Scene 5 (f=278..354): Fibonacci Vortex Finale
        double finT = (frame - 278) / FPS;
        drawStudioFinale(frame, finT, logo, uiReady, @"Look. Talk. Sent.", @"100% LOCAL · FREE & OPEN SOURCE");
    }

    if (blurX > 1 || blurY > 1) applyDirectionalMotionBlur(rep, blurX, blurY);

    if (frame >= 5 && frame <= 70) {
        drawSubtitlePill(@"Voice input.", @"Without a hotkey.", ease(frame, 5, 10) * (1.0 - ease(frame, 64, 70)));
    } else if (frame >= 74 && frame <= 124) {
        drawSubtitlePill(@"It can see you", @"looking at the screen.", ease(frame, 74, 80) * (1.0 - ease(frame, 118, 124)));
    } else if (frame >= 126 && frame <= 207) {
        drawSubtitlePill(@"So you just talk.", @"No button to hold.", ease(frame, 126, 132) * (1.0 - ease(frame, 201, 207)));
    } else if (frame >= 209 && frame <= 276) {
        drawSubtitlePill(@"Then it types it in,", @"and presses Enter.", ease(frame, 209, 215) * (1.0 - ease(frame, 270, 276)));
    }
}

// ============================================================================
// REEL 2: 10.0s (300 frames) "Speaks Developer"
// VO: 07(5..106), 05(112..193), 02(222..294)
// ============================================================================
static void renderReel2Frame(int frame, NSBitmapImageRep *rep, NSImage *logo, NSImage *uiReady) {
    double t = frame / FPS;
    int blurX = 0;

    if (frame < 108) {
        // Scene 1 (Warm Ivory): Why Literal Dictation Fails Developers + Shatter
        drawPlateBackground(YES, 1.0, 0.0);
        drawHorizonGrid3D(frame, YES, 0.85, 0.0);

        drawText(@"WHISPER HEARS WORDS · SAYWHEN WRITES SYNTAX", 72, 200, fontMono(16, 700), C_MUTED_LT, 1.8);
        drawText(@"Speak human.", 72, 238, fontSans(74, 800), C_PLUM_INK, -2.5);
        NSFont *hf2 = fontSans(74, 800);
        CGSize hsz2 = measureText(@"Write developer.", hf2, -2.5);
        drawRoundedBox(62, 322, hsz2.width + 32, 90, 14, rgba(251, 179, 3, 0.30), nil, 0);
        drawText(@"Write developer.", 78, 326, hf2, C_PLUM_INK, -2.5);

        double boom = eo(seg(t, 2.95, 3.55));
        double fade = 1.0 - seg(t, 3.25, 3.55);
        double cFade = (boom > 0.02) ? clamp01(1.0 - boom * 3.0) : 1.0;

        drawTiltedProblemCard(538, 635, 860, 224, -1.8, @"LITERAL DICTATION", @"\"git checkout dash b main\"",
                              @"Types the words 'dash' and 'main' literally", @"x pathspec error!",
                              cFade, ease(t, 0.55, 0.95), 404u);
        drawTiltedProblemCard(546, 910, 860, 224, 1.4, @"KUBERNETES CLI", @"\"cubectal get pods dash n prod\"",
                              @"Mishears tool name and namespace flag", @"x command not found",
                              cFade * eo(seg(t, 0.45, 0.75)), ease(t, 1.15, 1.55), 505u);
        drawTiltedProblemCard(532, 1185, 860, 224, -1.5, @"GIT & NPM", @"\"get stash pot and ntm run build\"",
                              @"Breaks operators and subcommands", @"x syntax error",
                              cFade * eo(seg(t, 0.95, 1.25)), ease(t, 1.75, 2.15), 606u);

        drawShatteringKeycaps3D(boom, fade, CX, 910);
    } else if (frame < 220) {
        // Scene 2 (Dark Plum): Local AST Compiler Scanning Beam + Exact CLI Output
        double s2T = (frame - 108) / FPS;
        if (frame < 114) blurX = (114 - frame) * 4;

        drawPlateBackground(NO, 1.0, 0.0);
        drawAcousticTerrain3D(frame, 1520.0, NO, 0.85);

        drawText(@"LOCAL RUST AST PASS · SRC-TAURI/SRC/CLEANUP.RS", 72, 200, fontMono(16, 700), C_AMBER, 1.8);
        drawText(@"34 / 34 exact.", 72, 240, fontSerif(104, YES), C_IVORY, -2.0);

        struct { NSString *tag; NSString *raw; NSString *cli; double at; } diffs[3] = {
            { @"01 · FLAGS & END PHRASE", @"\"um cargo test dash dash release send it\"", @"$ cargo test --release", 0.10 },
            { @"02 · K8S CUSTOM VOCAB", @"\"cubectal get pods dash n prod\"", @"$ kubectl get pods -n prod", 0.95 },
            { @"03 · GIT & SHELL OPS", @"\"get stash pot and ntm run build\"", @"$ git stash pop && npm run build", 1.80 }
        };

        for (int i = 0; i < 3; i++) {
            double u = eo(seg(s2T, diffs[i].at, diffs[i].at + 0.32));
            BOOL done = (s2T >= diffs[i].at + 0.48);
            CGFloat py = 415 + i * 290 + (1.0 - u) * 20.0;

            drawShadowRoundedBox(68, py, W - 136, 256, 24,
                                 done ? rgba(44, 12, 38, 0.96) : rgba(24, 6, 22, 0.92),
                                 done ? C_AMBER : rgba(255, 255, 255, 0.16), done ? 2.4 : 1.5,
                                 rgba(0, 0, 0, 0.48), 32.0, 14.0);
            drawText(diffs[i].tag, 102, py + 26, fontMono(15, 700), done ? C_AMBER : C_MUTED_DK, 1.6);

            drawRoundedBox(W - 266, py + 22, 164, 34, 17, done ? rgba(61, 220, 151, 0.18) : rgba(255, 255, 255, 0.08), nil, 0);
            drawText(done ? @"OK COMPILED" : @"STREAMING", W - 246, py + 29, fontMono(14, 700), done ? C_EMERALD : C_SILVER, 0.8);

            drawText(diffs[i].raw, 102, py + 82, fontMono(21, 500), C_MUTED_DK, -0.2);

            if (done) {
                NSArray<NSString *> *cliArr = @[ diffs[i].cli ];
                drawWetInkLines(cliArr, s2T, diffs[i].at + 0.48, 52.0, 0.40, 102, py + 148, 44,
                                fontMono(33, 800), 246, 238, 244, 251, 179, 3, NO);
                // Laser scan line flash right after compilation triggers
                double scanP = seg(s2T, diffs[i].at + 0.44, diffs[i].at + 0.78);
                if (scanP > 0.0 && scanP < 1.0) {
                    CGFloat sx = 72.0 + (W - 144.0) * scanP;
                    drawRoundedBox(sx, py + 10, 4.0, 236, 2.0, rgba(251, 179, 3, 0.85 * (1.0 - scanP)), nil, 0);
                }
            } else {
                drawText(@"$ ...", 102, py + 148, fontMono(33, 700), C_MUTED_DK, 0.0);
            }
        }

        double scU = eo(seg(s2T, 2.35, 2.85));
        if (scU > 0.01) {
            drawRoundedBox(68, 1300, W - 136, 92, 20, rgba(61, 220, 151, 0.15 * scU), C_EMERALD, 2.0);
            drawText(@"OK  0 ms cloud · Deterministically tested in cleanup.rs", 104, 1330, fontMono(23, 700), C_EMERALD, -0.3);
        }

        drawLightSpeedWarp(s2T, 2.25, CX, CY);
        drawPixelBurst(s2T, 2.35, CX, 1340);
    } else {
        // Scene 3: Fibonacci Vortex Finale
        double finT = (frame - 220) / FPS;
        drawStudioFinale(frame, finT, logo, uiReady, @"Speaks developer.", @"NO LLM REQUIRED · CUSTOM VOCAB");
    }

    if (blurX > 1) applyDirectionalMotionBlur(rep, blurX, 0);

    if (frame >= 5 && frame <= 106) {
        drawSubtitlePill(@"You speak like a person.", @"It writes like a developer.", ease(frame, 5, 11) * (1.0 - ease(frame, 100, 106)));
    } else if (frame >= 112 && frame <= 193) {
        drawSubtitlePill(@"It writes the real command,", @"and lets you check.", ease(frame, 112, 118) * (1.0 - ease(frame, 187, 193)));
    }
}

// ============================================================================
// REEL 3: 10.0s (300 frames) "Safe & Private by Design"
// VO: 09(6..51), 10(78..152), 11(168..214), 12(222..294)
// ============================================================================
static void renderReel3Frame(int frame, NSBitmapImageRep *rep, NSImage *logo, NSImage *uiDestructive) {
    double t = frame / FPS;
    int blurX = 0, blurY = 0;

    if (frame < 76) {
        // Scene 1 (Warm Ivory): Always Cancelable Preview + Shake/Esc Abort Burst
        drawPlateBackground(YES, 1.0, 0.0);
        drawHorizonGrid3D(frame, YES, 0.85, 0.0);

        BOOL canceled = (t >= 1.25);
        drawText(@"01 · 100% CANCELABLE AUTO-ENTER", 72, 200, fontMono(18, 700), canceled ? C_DANGER_DK : C_MUTED_LT, 2.0);
        drawText(@"Every send can", 72, 240, fontSerif(102, YES), C_PLUM_INK, -2.0);
        drawText(@"be canceled.", 72, 342, fontSerif(102, YES), canceled ? C_DANGER_DK : C_AMBER_DARK, -2.0);

        drawShadowRoundedBox(68, 496, W - 136, 300, 28, C_WHITE, canceled ? C_DANGER_DK : C_AMBER, 2.8,
                             rgba(34, 6, 28, 0.16), 36.0, 16.0);
        if (!canceled) {
            drawText(@"SENDING TO TERMINAL IN 1.1S", 106, 534, fontMono(16, 700), C_AMBER_DARK, 1.6);
            drawText(@"cargo test --release", 106, 594, fontMono(40, 800), C_PLUM_INK, -0.6);
            drawRoundedBox(106, 706, (W - 212) * clamp01(1.0 - t * 0.65), 14, 7, C_AMBER, nil, 0);
        } else {
            drawText(@"x ABORTED BEFORE ENTER", 106, 534, fontMono(17, 700), C_DANGER_DK, 1.6);
            drawText(@"Canceled. Zero chars typed.", 106, 598, fontSans(40, 800), C_PLUM_INK, -1.0);
            drawText(@"Clipboard restored · Draft discarded", 106, 680, fontMono(21, 500), C_MUTED_LT, 0.0);
            drawPixelBurst(t, 1.25, CX, 640);
        }

        NSArray *ways = @[
            @[@"ESC", @"Tap Esc once during preview", @(canceled)],
            @[@"SHAKE", @"Shake head left-right once", @(NO)],
            @[@"VOICE", @"Say \"scratch that\" mid-stream", @(NO)]
        ];
        for (int i = 0; i < 3; i++) {
            NSArray *row = ways[i];
            BOOL on = [row[2] boolValue];
            CGFloat py = 848 + i * 160;
            drawShadowRoundedBox(68, py, W - 136, 130, 22, C_WHITE,
                                 on ? C_DANGER_DK : rgba(34, 6, 28, 0.12), on ? 2.4 : 1.5,
                                 rgba(34, 6, 28, 0.08), 22.0, 8.0);
            drawRoundedBox(98, py + 32, 136, 66, 14, on ? C_DANGER_DK : C_PLUM, nil, 0);
            CGSize kw = measureText(row[0], fontMono(22, 800), 0.5);
            drawText(row[0], 98 + (136 - kw.width) * 0.5, py + 50, fontMono(22, 800), C_WHITE, 0.5);
            drawText(row[1], 260, py + 47, fontSans(29, 700), C_PLUM_INK, -0.4);
        }
    } else if (frame < 166) {
        // Scene 2 (Dark Plum + Crimson Aura): 3-Ring HexShieldForcefield + Destructive Command Guard
        double s2T = (frame - 76) / FPS;
        if (frame < 82) blurX = (82 - frame) * 4;

        drawPlateBackground(NO, 1.0, 1.0);
        drawHorizonGrid3D(frame, NO, 0.85, 0.0);

        drawText(@"02 · DESTRUCTIVE COMMAND GUARD", 72, 200, fontMono(18, 700), C_DANGER, 2.0);
        drawText(@"Risky commands", 72, 240, fontSerif(100, YES), C_IVORY, -2.0);
        drawText(@"never press Enter.", 72, 340, fontSerif(100, YES), C_DANGER, -2.0);

        drawShadowRoundedBox(68, 492, W - 136, 360, 28, rgba(20, 4, 16, 0.96), C_DANGER, 2.4,
                             rgba(0, 0, 0, 0.55), 40.0, 18.0);
        drawText(@"DOWNGRADED TO PASTE-ONLY · ENTER LOCKED", 104, 530, fontMono(16, 700), C_DANGER, 1.5);

        NSArray<NSString *> *cmd = @[ @"$ rm -rf dist/", @"  && git reset --hard HEAD~1" ];
        drawWetInkLines(cmd, s2T, 0.05, 42.0, 0.40, 104, 594, 52, fontMono(32, 700), 246, 238, 244, 255, 107, 94, YES);

        // Shielded ENTER key cap on right side
        double shP = eo(seg(s2T, 0.45, 0.90));
        drawHexShieldForcefield(s2T, 0.45, CX, 770, 56.0);
        drawRoundedBox(104, 738, W - 208, 70, 18, rgba(255, 107, 94, 0.18 * shP), rgba(255, 107, 94, 0.7 * shP), 2.0);
        drawCenteredText(@"ENTER KEY WITHHELD · HUMAN APPROVAL REQUIRED", 760, fontMono(20, 700), rgba(255, 210, 206, shP), 0.4);

        drawCroppedImageCard(uiDestructive, 68, 892, W - 136, 624, 24, rgba(255, 107, 94, 0.48), 1.0, YES);
    } else if (frame < 226) {
        // Scene 3 (Warm Ivory via Halftone Iris -> Vertical Slot Reel): 100% On-Device Privacy
        double s3T = (frame - 166) / FPS;
        double irisR = lerpd(40.0, 1280.0, eo(seg(s3T, 0.0, 0.35)));

        drawPlateBackground(NO, 0.6, 0.0);
        [NSGraphicsContext saveGraphicsState];
        beginHalftoneIrisClip(irisR);

        drawPlateBackground(YES, 1.0, 0.0);
        drawHorizonGrid3D(frame, YES, 0.9, 0.0);
        drawCenteredText(@"03 · ZERO-NETWORK PRIVACY", 280, fontMono(18, 700), C_MUTED_LT, 2.2);

        NSArray<NSString *> *privItems = @[
            @"Camera In RAM.",
            @"0 Frames Saved.",
            @"Silero VAD Local.",
            @"Whisper Metal GPU.",
            @"Zero Telemetry.",
            @"Nothing Leaves Your Mac."
        ];
        double vel = 0.0, landPop = 0.0;
        drawVerticalSlotReel(s3T, CY - 50, YES, privItems, &vel, &landPop);
        if (vel > 12.0) blurY = (int)fmin(14.0, vel * 0.14);

        drawCenteredText(@"Reads attention in memory, then discards every frame", 1410, fontSans(24, 600), C_MUTED_LT, -0.3);

        [NSGraphicsContext restoreGraphicsState];
        if (irisR < 1260.0) {
            drawHalftoneIrisRing(irisR, 130.0, 1.0 - seg(irisR, 980.0, 1260.0));
        }
    } else {
        // Scene 4 (Dark Plum + Fibonacci Vortex Finale)
        double finT = (frame - 226) / FPS;
        drawStudioFinale(frame, finT, logo, uiDestructive, @"Safe. Local. Hands-free.", @"0 BYTES SENT · ON-DEVICE PRIVACY");
    }

    if (blurX > 1 || blurY > 1) applyDirectionalMotionBlur(rep, blurX, blurY);

    if (frame >= 6 && frame <= 54) {
        drawSubtitlePill(@"And you can", @"always stop it.", ease(frame, 6, 12) * (1.0 - ease(frame, 48, 54)));
    } else if (frame >= 78 && frame <= 156) {
        drawSubtitlePill(@"Risky commands", @"never press Enter.", ease(frame, 78, 84) * (1.0 - ease(frame, 150, 156)));
    } else if (frame >= 168 && frame <= 218) {
        drawSubtitlePill(@"Nothing leaves", @"this computer.", ease(frame, 168, 174) * (1.0 - ease(frame, 212, 218)));
    }
}

// ============================================================================
// ENTRYPOINT
// ============================================================================
int main(int argc, const char * argv[]) {
    @autoreleasepool {
        if (argc < 2) {
            fprintf(stderr, "Usage: render_reels <hero|1|2|3>\n");
            return 1;
        }

        const char *modeStr = argv[1];
        int reelNum = 0;
        int totalFrames = 450;
        const char *audioPath = "film/public/reel-hero-audio.wav";

        if (strcmp(modeStr, "1") == 0) {
            reelNum = 1;
            totalFrames = 354;
            audioPath = "film/public/reel-1-audio.wav";
        } else if (strcmp(modeStr, "2") == 0) {
            reelNum = 2;
            totalFrames = 300;
            audioPath = "film/public/reel-2-audio.wav";
        } else if (strcmp(modeStr, "3") == 0) {
            reelNum = 3;
            totalFrames = 300;
            audioPath = "film/public/reel-3-audio.wav";
        }

        loadWavEnvelope(audioPath, totalFrames);
        registerBundledFonts();

        NSImage *logo = [[NSImage alloc] initWithContentsOfFile:@"film/public/logo.png"];
        NSImage *uiReady = [[NSImage alloc] initWithContentsOfFile:@"film/public/screens/ui-dark-ready.png"];
        NSImage *uiListening = [[NSImage alloc] initWithContentsOfFile:@"film/public/screens/ui-dark-listening.png"];
        NSImage *uiDestructive = [[NSImage alloc] initWithContentsOfFile:@"film/public/screens/ui-dark-destructive.png"];

        NSBitmapImageRep *rep = [[NSBitmapImageRep alloc]
            initWithBitmapDataPlanes:NULL
                          pixelsWide:(NSInteger)W
                          pixelsHigh:(NSInteger)H
                       bitsPerSample:8
                     samplesPerPixel:4
                            hasAlpha:YES
                            isPlanar:NO
                      colorSpaceName:NSDeviceRGBColorSpace
                        bitmapFormat:0
                         bytesPerRow:(NSInteger)W * 4
                        bitsPerPixel:32];

        NSGraphicsContext *ctx = [NSGraphicsContext graphicsContextWithBitmapImageRep:rep];

        for (int f = 0; f < totalFrames; f++) {
            @autoreleasepool {
                [NSGraphicsContext setCurrentContext:ctx];
                [ctx setShouldAntialias:YES];
                [ctx setImageInterpolation:NSImageInterpolationHigh];

                if (reelNum == 0) {
                    renderReelHeroFrame(f, rep, logo, uiReady, uiListening);
                } else if (reelNum == 1) {
                    renderReel1Frame(f, rep, logo, uiReady);
                } else if (reelNum == 2) {
                    renderReel2Frame(f, rep, logo, uiReady);
                } else {
                    renderReel3Frame(f, rep, logo, uiDestructive);
                }

                [ctx flushGraphics];
                fwrite([rep bitmapData], 1, (size_t)(W * H * 4), stdout);
            }
        }
    }
    return 0;
}

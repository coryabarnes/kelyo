import { useMemo, useRef, useState } from "react";
import { View } from "react-native";
import type { PowerUpProps, ThemeSwatchProps, WindowBackdropProps } from "@ardia-real/shaders";
import { useBrowser } from "../../../browser/src/state/browser/store";
import { prefersReducedMotion } from "./web/gl/clock";
import { parseHex } from "./web/gl/colors";
import { GLView } from "./web/gl/GLView";
import { backdropFragment, haloFragment, powerUpFragment } from "./web/gl/sources";
import { webgl2Supported } from "./web/gl/surface";
import { backdropUniforms, HALO, haloUniforms, POWER_UP, powerUpPalette, powerUpUniforms } from "./web/gl/uniforms";
import { BackdropFallback, HaloFallback, PowerUpFallback } from "./web/gl/fallbacks";
import { Vibrancy } from "./web/gl/Vibrancy";
import { usePageHoleLayer } from "../shellAbove";

export * from "@ardia-real/shaders";

const FRAME = 1 / 60;

export function WindowBackdrop({ colors, angle = 180, grainOpacity = 0.09, grainScale = 1, vibrancy, tintColor, tintAlpha, tintLightness, style }: WindowBackdropProps) {
  const dark = useBrowser((s) => s.ui.appDark);
  const key = `${colors.join()}|${angle}|${grainOpacity}|${grainScale}`;
  const uniformsAt = useMemo(() => (css: [number, number]) => backdropUniforms({ colors, angle, grainOpacity, grainScale }, css), [key]);
  const layer = useRef<View & HTMLElement>(null);
  usePageHoleLayer(layer);
  return (
    <View ref={layer} pointerEvents="none" style={style}>
      {vibrancy ? (
        <Vibrancy dark={dark} tintColor={tintColor} tintAlpha={tintAlpha} tintLightness={tintLightness} />
      ) : webgl2Supported() ? (
        <GLView fragment={backdropFragment} uniformsAt={uniformsAt} version={key} />
      ) : (
        <BackdropFallback colors={colors} angle={angle} />
      )}
    </View>
  );
}

export function PowerUp({ palette = "pink", speed = POWER_UP.defaultSpeed, origin = POWER_UP.defaultOrigin, cornerRadius = POWER_UP.defaultCornerRadius, halo = null, style }: PowerUpProps) {
  const dark = useBrowser((s) => s.ui.appDark);
  const [reduceMotion] = useState(prefersReducedMotion);
  const paletteKey = typeof palette === "string" ? palette : palette.join();
  const srgb = useMemo(() => powerUpPalette(palette), [paletteKey]);
  const powerUpAt = useMemo(() => (_css: [number, number], time: number) => powerUpUniforms({ srgb, speed, origin }, time), [srgb, speed, origin]);
  const haloAt = useMemo(
    () => (css: [number, number], time: number) => haloUniforms({ srgb, speed, cornerRadius, dark }, css, time),
    [srgb, speed, cornerRadius, dark],
  );
  if (reduceMotion) return <View pointerEvents="none" style={style} />;
  const gl = webgl2Supported();
  const haloBox = halo && halo.width > 0 && halo.height > 0
    ? { left: halo.x - HALO.inset, top: halo.y - HALO.inset, width: halo.width + 2 * HALO.inset, height: halo.height + 2 * HALO.inset }
    : null;
  return (
    <View pointerEvents="none" style={style}>
      {gl ? (
        <GLView fragment={powerUpFragment} uniformsAt={powerUpAt} animation={{ lastFrameTime: POWER_UP.finishTime - FRAME }} style={{ filter: "blur(24px)" }} />
      ) : (
        <PowerUpFallback srgb={srgb} speed={speed} />
      )}
      {haloBox ? (
        <div style={{ position: "absolute", pointerEvents: "none", ...haloBox }}>
          {gl ? (
            <GLView fragment={haloFragment} uniformsAt={haloAt} animation={{ lastFrameTime: HALO.finishTime }} />
          ) : (
            <HaloFallback color={srgb[0] ?? parseHex("#FF845A")!} speed={speed} cornerRadius={cornerRadius + 2} dark={dark} />
          )}
        </div>
      ) : null}
    </View>
  );
}

const swatchColor = (colors: number[], index: number) => {
  const [r = 0, g = 0, b = 0, a = 1] = colors.slice(index * 4, index * 4 + 4);
  return `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)} / ${a})`;
};

function huePad(lightness: number) {
  const l = Math.round(lightness * 100);
  const hues = Array.from({ length: 13 }, (_, i) => `hsl(${i * 30} 100% ${l}%)`).join(", ");
  return `radial-gradient(closest-side, hsl(0 0% ${l}%), hsl(0 0% ${l}% / 0)), conic-gradient(from 90deg, ${hues})`;
}

export function ThemeSwatch({ colors, angle = 135, padLightness = -1, style }: ThemeSwatchProps) {
  if (colors.length === 0 && padLightness >= 0) {
    return (
      <View pointerEvents="none" style={[{ overflow: "hidden", borderRadius: 999 }, style]}>
        <div style={{ position: "absolute", inset: 0, background: huePad(padLightness) }} />
      </View>
    );
  }
  const count = Math.max(1, Math.floor(colors.length / 4));
  const stops = Array.from({ length: count }, (_, i) => swatchColor(colors, i));
  const background = count === 1 ? stops[0] : `linear-gradient(in oklab ${angle}deg, ${stops.join(", ")})`;
  return (
    <View pointerEvents="none" style={[{ overflow: "hidden", borderRadius: 999 }, style]}>
      <div style={{ position: "absolute", inset: 0, background }} />
    </View>
  );
}


import { Circle, CircleProps } from "@motion-canvas/2d";

// Catppuccin Mocha colors
const COLORS = {
  red: "#f38ba8",
  green: "#a6e3a1",
  blue: "#89b4fa",
  background: "#1e1e2e",
};

export type QuarkColor = "red" | "green" | "blue";
export type QuarkFlavor = "up" | "down";

export interface QuarkProps extends CircleProps {
  quarkColor: QuarkColor;
  quarkFlavor: QuarkFlavor;
}

export class Quark extends Circle {
  private _quarkColor: QuarkColor;
  private readonly _quarkFlavor: QuarkFlavor;

  public constructor(props: QuarkProps) {
    const quarkSize = typeof props.size === "number" ? props.size : 120;
    const colorHex = COLORS[props.quarkColor];
    const isDown = props.quarkFlavor === "down";

    super({
      ...props,
      size: quarkSize,
      fill: colorHex,
      shadowBlur: 40,
      shadowColor: colorHex,
      compositeOperation: isDown ? "source-over" : undefined,
    });

    this._quarkColor = props.quarkColor;
    this._quarkFlavor = props.quarkFlavor;

    if (isDown) {
      this.add(
        <Circle
          size={quarkSize * 0.4}
          fill={COLORS.background}
          compositeOperation="destination-out"
        />,
      );
    }
  }

  public get quarkColor(): QuarkColor {
    return this._quarkColor;
  }

  public get quarkFlavor(): QuarkFlavor {
    return this._quarkFlavor;
  }

  /**
   * Update the quark's displayed color (fill + glow) at runtime.
   * This is used by higher-level animations (e.g. hadron color rotation).
   */
  public setQuarkColor(quarkColor: QuarkColor) {
    this._quarkColor = quarkColor;

    const colorHex = COLORS[quarkColor];
    this.fill(colorHex);
    this.shadowColor(colorHex);
  }
}

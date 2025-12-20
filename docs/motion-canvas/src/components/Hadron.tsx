import { Circle, Layout, LayoutProps, Line, Txt } from "@motion-canvas/2d";
import {
  all,
  createRef,
  easeInOutCubic,
  easeOutCubic,
  linear,
  loop,
  waitFor,
  waitUntil,
  spawn,
} from "@motion-canvas/core";
import { Quark, QuarkColor, QuarkFlavor } from "./Quark";

const COLORS = {
  red: "#f38ba8",
  green: "#a6e3a1",
  blue: "#89b4fa",
  text: "#cdd6f4",
};

export type HadronType = "proton" | "neutron";

export interface HadronConfig {
  quark1: { color: QuarkColor; flavor: QuarkFlavor };
  quark2: { color: QuarkColor; flavor: QuarkFlavor };
  quark3: { color: QuarkColor; flavor: QuarkFlavor };
}

export const HADRON_CONFIGS: Record<HadronType, HadronConfig> = {
  proton: {
    quark1: { color: "red", flavor: "up" },
    quark2: { color: "green", flavor: "up" },
    quark3: { color: "blue", flavor: "down" },
  },
  neutron: {
    quark1: { color: "red", flavor: "up" },
    quark2: { color: "green", flavor: "down" },
    quark3: { color: "blue", flavor: "down" },
  },
};

export interface HadronProps extends LayoutProps {
  hadronType: HadronType;
  quarkSize?: number;
  radius?: number;
  startDistance?: number;

  /**
   * Whether to render the bottom text block (equation -> label -> composition).
   * If disabled, no text elements are created and the related animation helpers no-op.
   */
  showText?: boolean;

  /**
   * Baseline Y for the text block (equation line). Increase to move text further down.
   * Defaults to a value that avoids colliding with the quarks during formation.
   */
  textBaseY?: number;
}

export class Hadron extends Layout {
  public hadronType: HadronType;

  public readonly quark1 = createRef<Quark>();
  public readonly quark2 = createRef<Quark>();
  public readonly quark3 = createRef<Quark>();

  public readonly bond12 = createRef<Line>();
  public readonly bond23 = createRef<Line>();
  public readonly bond31 = createRef<Line>();

  public readonly shell = createRef<Circle>();

  // Bottom text block (optional)
  public readonly equation = createRef<Txt>();
  public readonly label = createRef<Txt>();
  public readonly composition = createRef<Txt>();

  private quarkSize: number;
  private radius: number;
  private startDistance: number;
  private showText: boolean;
  private textBaseY: number;

  private getComposition(hadronType: HadronType): string {
    switch (hadronType) {
      case "proton":
        return "uud";
      case "neutron":
        return "udd";
      default:
        return "";
    }
  }

  private getBaseEquation(hadronType: HadronType): string {
    switch (hadronType) {
      case "proton":
        return "2 Up + 1 Down";
      case "neutron":
        return "1 Up + 2 Down";
      default:
        return "";
    }
  }

  private getHadronName(hadronType: HadronType): string {
    return hadronType.charAt(0).toUpperCase() + hadronType.slice(1);
  }

  private getFullEquation(hadronType: HadronType): string {
    return `${this.getBaseEquation(hadronType)} = ${this.getHadronName(
      hadronType,
    )}`;
  }

  public constructor(props: HadronProps) {
    super(props);

    this.hadronType = props.hadronType;
    this.quarkSize = props.quarkSize ?? 120;
    this.radius = props.radius ?? 400;
    this.startDistance = props.startDistance ?? 800;

    this.showText = props.showText ?? true;
    this.textBaseY = props.textBaseY ?? 600;

    const config = HADRON_CONFIGS[props.hadronType];
    const shellColor =
      props.hadronType === "proton" ? COLORS.green : COLORS.blue;

    // Get hadron name and composition
    const hadronName = this.getHadronName(props.hadronType);
    const composition = this.getComposition(props.hadronType);

    this.add(
      <>
        {/* Bonds */}
        <Line
          ref={this.bond12}
          stroke={COLORS.text}
          lineWidth={8}
          opacity={0}
          lineDash={[20, 20]}
          points={null}
        />
        <Line
          ref={this.bond23}
          stroke={COLORS.text}
          lineWidth={8}
          opacity={0}
          lineDash={[20, 20]}
          points={null}
        />
        <Line
          ref={this.bond31}
          stroke={COLORS.text}
          lineWidth={8}
          opacity={0}
          lineDash={[20, 20]}
          points={null}
        />

        {/* Shell */}
        <Circle
          ref={this.shell}
          size={this.radius * 2}
          stroke={shellColor}
          lineWidth={20}
          opacity={0}
        />

        {/* Quarks */}
        <Quark
          ref={this.quark1}
          quarkColor={config.quark1.color}
          quarkFlavor={config.quark1.flavor}
          size={this.quarkSize}
          x={-this.startDistance}
          y={-this.startDistance * 0.5}
        />
        <Quark
          ref={this.quark2}
          quarkColor={config.quark2.color}
          quarkFlavor={config.quark2.flavor}
          size={this.quarkSize}
          x={this.startDistance}
          y={-this.startDistance * 0.5}
        />
        <Quark
          ref={this.quark3}
          quarkColor={config.quark3.color}
          quarkFlavor={config.quark3.flavor}
          size={this.quarkSize}
          y={this.startDistance * 0.7}
        />

        {/* Bottom text block */}
        {this.showText && (
          <>
            <Txt
              ref={this.equation}
              text=""
              fontSize={60}
              fontFamily={"monospace"}
              fill={COLORS.text}
              y={this.textBaseY}
              opacity={1}
            />

            <Txt
              ref={this.composition}
              fontSize={60}
              fontFamily={"monospace"}
              y={this.textBaseY + 80}
              opacity={0}
              text=""
            />
          </>
        )}
      </>,
    );
  }

  public *animateFormation(duration: number = 2) {
    const finalPositions = {
      q1: { x: -this.radius * 0.4, y: -this.radius * 0.3 },
      q2: { x: this.radius * 0.4, y: -this.radius * 0.3 },
      q3: { x: 0, y: this.radius * 0.5 },
    };

    // Move quarks together
    yield* all(
      this.quark1().position.x(finalPositions.q1.x, duration, easeInOutCubic),
      this.quark1().position.y(finalPositions.q1.y, duration, easeInOutCubic),
      this.quark2().position.x(finalPositions.q2.x, duration, easeInOutCubic),
      this.quark2().position.y(finalPositions.q2.y, duration, easeInOutCubic),
      this.quark3().position.x(finalPositions.q3.x, duration, easeInOutCubic),
      this.quark3().position.y(finalPositions.q3.y, duration, easeInOutCubic),
      this.bond12().opacity(0.6, duration * 0.75, easeOutCubic),
      this.bond23().opacity(0.6, duration * 0.75, easeOutCubic),
      this.bond31().opacity(0.6, duration * 0.75, easeOutCubic),
    );

    // Update bond positions
    this.updateBondPositions();
  }

  public updateBondPositions() {
    this.bond12().points([this.quark1().position(), this.quark2().position()]);
    this.bond23().points([this.quark2().position(), this.quark3().position()]);
    this.bond31().points([this.quark3().position(), this.quark1().position()]);
  }

  public *showShell(duration: number = 1) {
    yield* this.shell().opacity(0.6, duration, easeOutCubic);
  }

  /**
   * Run the full "hadron formation" beat sequence used by the quark formation scene.
   *
   * This exists to avoid duplicating the same timeline logic for proton + neutron.
   * The scene should be responsible only for:
   * - creating hadrons
   * - calling this helper with a prefix (e.g. "P" / "N")
   * - doing any cross-hadron choreography (scaling/moving/fading groups, etc.)
   *
   * This helper will:
   * - wait for the prefixed time events
   * - fade in the hadron
   * - animate quark formation
   * - type equation (base -> full)
   * - start bond animation
   * - show shell
   * - collapse equation to label+composition
   * - rotate displayed quark colors
   */
  public *animateSequence(options: {
    prefix: string;
    formationSeconds?: number;
    equationDelaySeconds?: number;
    baseEquationSeconds?: number;
    fullEquationSeconds?: number;
    collapseSeconds?: number;
    colorRotationDelaySeconds?: number;
    colorRotationCycles?: number;
    postColorsHoldSeconds?: number;
    bondSpeed?: number;
  }): Generator<any, void, any> {
    const {
      prefix,
      formationSeconds = 2,
      equationDelaySeconds = 0.5,
      baseEquationSeconds = 1,
      fullEquationSeconds = 1,
      collapseSeconds = 0.8,
      colorRotationDelaySeconds = 0.5,
      colorRotationCycles = 1,
      postColorsHoldSeconds = 2,
      bondSpeed = 200,
    } = options;

    // 1) Show
    yield* waitUntil(`${prefix}: Show`);
    yield* this.opacity(1, 0.5);

    // 2) Quarks (formation is concurrent with equation typing)
    yield* waitUntil(`${prefix}: Quarks`);
    const formationTask = yield this.animateFormation(formationSeconds);

    yield* waitFor(equationDelaySeconds);
    yield* waitUntil(`${prefix}: Equation`);
    yield* this.typeBaseEquation(baseEquationSeconds);

    yield* formationTask;
    this.updateBondPositions();

    // 3) Bonds + shell + full equation
    yield* waitUntil(`${prefix}: Bonds`);
    spawn(this.animateBonds(bondSpeed));

    yield* waitUntil(`${prefix}: Shell`);
    yield* all(this.showShell(1), this.typeFullEquation(fullEquationSeconds));
    yield* waitFor(1);

    // 4) Collapse to label
    yield* waitUntil(`${prefix}: Label`);
    yield* this.collapseToLabel(collapseSeconds);

    // 5) Rotate colors
    yield* waitUntil(`${prefix}: Colors`);
    yield* this.rotateQuarkColors(
      colorRotationDelaySeconds,
      colorRotationCycles,
    );
    yield* waitFor(postColorsHoldSeconds);
  }

  public *animateBonds(speed: number = 200) {
    // Ensure there's a dash pattern set (without this, dash offset does nothing).
    this.bond12().lineDash([20, 20]);
    this.bond23().lineDash([20, 20]);
    this.bond31().lineDash([20, 20]);

    // Motion Canvas `loop()` is a generator that must be run on a separate thread
    // to keep going forever while the main timeline continues.
    //
    // Also: `lineDashOffset` needs to *accumulate* over time. We do that by
    // incrementing an offset every frame.
    let offset = this.bond12().lineDashOffset();

    yield* all(
      loop(Infinity, () => {
        offset -= speed * (1 / 60);

        this.bond12().lineDashOffset(offset);
        this.bond23().lineDashOffset(offset);
        this.bond31().lineDashOffset(offset);

        // Advance one frame.
        return;
      }),
    );
  }

  public *showComposition(
    name: string,
    _composition: string,
    duration: number = 0.8,
  ) {
    if (!this.showText) return;
    yield* this.composition().opacity(1, 0.5, easeOutCubic);
  }

  /**
   * Stage 1: type the base equation (e.g. "2 Up + 1 Down") onto the equation line.
   */
  public *typeBaseEquation(duration: number = 1) {
    if (!this.showText) return;

    yield* this.equation().opacity(1, 0);
    yield* this.equation().text(
      this.getBaseEquation(this.hadronType),
      duration,
    );
  }

  /**
   * Stage 2: type the full equation including the result (e.g. "... = Proton") onto the equation line.
   * Implemented as an in-place text change so it looks like continued typing.
   */
  public *typeFullEquation(duration: number = 1) {
    if (!this.showText) return;

    // Continue typing effect by transitioning text to the longer string.
    yield* this.equation().text(
      this.getFullEquation(this.hadronType),
      duration,
    );
  }

  /**
   * Stage 3: collapse from equation into label+composition:
   * - delete equation (typing deletion)
   * - fade in label
   * - fade in colored composition under it
   */
  public *collapseToLabel(duration: number = 0.9) {
    if (!this.showText) return;

    const hadronName = this.getHadronName(this.hadronType);

    // Delete only the prefix ("<base equation> = ") while keeping the hadron name.
    yield* this.equation().text(hadronName, duration);

    // Type the quark abbreviation using per-letter colored nodes from the start.
    const config = HADRON_CONFIGS[this.hadronType];
    const abbreviations = this.getComposition(this.hadronType);

    // Reset composition for a clean "type-in" each time.
    this.composition().removeChildren();
    this.composition().text("", 0);
    this.composition().add(
      <>
        <Txt
          text={abbreviations[0]}
          fill={COLORS[config.quark1.color]}
          opacity={0}
        />
        <Txt
          text={abbreviations[1]}
          fill={COLORS[config.quark2.color]}
          opacity={0}
        />
        <Txt
          text={abbreviations[2]}
          fill={COLORS[config.quark3.color]}
          opacity={0}
        />
      </>,
    );

    yield* this.composition().opacity(1, 0.15, easeOutCubic);

    const letters = this.composition().children().slice(0, 3) as Txt[];
    const perChar = 0.6 / 3;
    yield* letters[0].opacity(1, perChar, easeOutCubic);
    yield* letters[1].opacity(1, perChar, easeOutCubic);
    yield* letters[2].opacity(1, perChar, easeOutCubic);
  }

  private setCompositionLetterColors(
    colors: [QuarkColor, QuarkColor, QuarkColor],
  ) {
    if (!this.showText) return;

    const letters = this.composition().children().slice(0, 3) as Txt[];
    if (letters.length < 3) return;

    letters[0].fill(COLORS[colors[0]]);
    letters[1].fill(COLORS[colors[1]]);
    letters[2].fill(COLORS[colors[2]]);
  }

  private setQuarkColors(colors: [QuarkColor, QuarkColor, QuarkColor]) {
    this.quark1().setQuarkColor(colors[0]);
    this.quark2().setQuarkColor(colors[1]);
    this.quark3().setQuarkColor(colors[2]);
  }

  /**
   * Rotate quark colors (and the composition text letter colors) synchronously through
   * all 3! permutations.
   *
   * - `delaySeconds`: pause between each color change
   * - `cycles`: how many full permutation cycles to run (defaults to 1)
   *
   * Note: this cycles the *displayed* colors only; it does not change the hadron config/flavors.
   */
  public *rotateQuarkColors(delaySeconds: number = 0.5, cycles: number = 1) {
    const permutations: Array<[QuarkColor, QuarkColor, QuarkColor]> = [
      ["red", "green", "blue"],
      ["red", "blue", "green"],
      ["green", "red", "blue"],
      ["green", "blue", "red"],
      ["blue", "red", "green"],
      ["blue", "green", "red"],
    ];

    const total = Math.max(0, Math.floor(cycles));
    for (let c = 0; c < total; c++) {
      for (const p of permutations) {
        this.setQuarkColors(p);
        this.setCompositionLetterColors(p);
        yield* waitFor(delaySeconds);
      }
    }
  }
}

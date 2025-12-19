import { Circle, Line, makeScene2D, Txt } from "@motion-canvas/2d";
import {
  all,
  createRef,
  easeInOutCubic,
  easeOutCubic,
  linear,
  loop,
  waitFor,
  waitUntil,
} from "@motion-canvas/core";

// Catppuccin Mocha colors (matching the simulation)
const COLORS = {
  red: "#f38ba8",
  green: "#a6e3a1",
  blue: "#89b4fa",
  background: "#1e1e2e",
  text: "#cdd6f4",
};

export default makeScene2D(function* (view) {
  view.fill(COLORS.background);

  // Create references for the three quarks
  const quarkRed = createRef<Circle>();
  const quarkGreen = createRef<Circle>();
  const quarkBlue = createRef<Circle>();

  // Create references for the bonds between quarks
  const bondRedGreen = createRef<Line>();
  const bondGreenBlue = createRef<Line>();
  const bondBlueRed = createRef<Line>();

  // Create reference for the proton shell
  const protonShell = createRef<Circle>();

  // Create reference for title text
  const titleText = createRef<Txt>();

  // Quark size (bigger, like Duplo)
  const quarkSize = 120;
  const protonRadius = 400;

  // Initial positions (quarks start spread out, filling more space)
  const startDistance = 800;

  // Final positions (quarks in proton)
  const finalPositions = {
    red: { x: -protonRadius * 0.4, y: -protonRadius * 0.3 },
    green: { x: protonRadius * 0.4, y: -protonRadius * 0.3 },
    blue: { x: 0, y: protonRadius * 0.5 },
  };

  view.add(
    <>
      {/* Title */}
      <Txt
        ref={titleText}
        fontSize={72}
        fontFamily={"monospace"}
        fill={COLORS.text}
        y={-600}
        opacity={0}
      />

      {/* Bonds (drawn behind quarks) */}
      <Line
        ref={bondRedGreen}
        stroke={COLORS.text}
        lineWidth={8}
        opacity={0}
        lineDash={[20, 20]}
      />
      <Line
        ref={bondGreenBlue}
        stroke={COLORS.text}
        lineWidth={8}
        opacity={0}
        lineDash={[20, 20]}
      />
      <Line
        ref={bondBlueRed}
        stroke={COLORS.text}
        lineWidth={8}
        opacity={0}
        lineDash={[20, 20]}
      />

      {/* Proton shell (invisible initially) */}
      <Circle
        ref={protonShell}
        size={protonRadius * 2}
        stroke={COLORS.green}
        lineWidth={10}
        opacity={0}
      />

      {/* Red quark (Up quark) - solid sphere */}
      <Circle
        ref={quarkRed}
        size={quarkSize}
        fill={COLORS.red}
        x={-startDistance}
        y={-startDistance * 0.5}
        shadowBlur={40}
        shadowColor={COLORS.red}
      />

      {/* Green quark (Up quark) - solid sphere */}
      <Circle
        ref={quarkGreen}
        size={quarkSize}
        fill={COLORS.green}
        x={startDistance}
        y={-startDistance * 0.5}
        shadowBlur={40}
        shadowColor={COLORS.green}
      />

      {/* Blue quark (Down quark) - hollow center to distinguish from Up */}
      <Circle
        ref={quarkBlue}
        size={quarkSize}
        fill={COLORS.blue}
        y={startDistance * 0.7}
        shadowBlur={40}
        shadowColor={COLORS.blue}
        compositeOperation={"source-over"}
      >
        {/* Inner hole for Down quark - using stroke instead of fill for true transparency */}
        <Circle
          size={quarkSize * 0.4}
          fill={COLORS.background}
          compositeOperation={"destination-out"}
        />
      </Circle>
    </>,
  );

  // Animation sequence using waitUntil for timeline events
  yield* waitUntil("Title Fade In");
  yield* all(
    titleText().text("Three Quarks", 1),
    titleText().opacity(1, 1, easeOutCubic),
  );

  yield* waitUntil("Show Quark Types");
  yield* titleText().text("2 Up + 1 Down", 1.5);

  yield* waitUntil("Quarks Move Together");
  yield* all(
    quarkRed().position.x(finalPositions.red.x, 2, easeInOutCubic),
    quarkRed().position.y(finalPositions.red.y, 2, easeInOutCubic),
    quarkGreen().position.x(finalPositions.green.x, 2, easeInOutCubic),
    quarkGreen().position.y(finalPositions.green.y, 2, easeInOutCubic),
    quarkBlue().position.x(finalPositions.blue.x, 2, easeInOutCubic),
    quarkBlue().position.y(finalPositions.blue.y, 2, easeInOutCubic),
    bondRedGreen().opacity(0.6, 1.5, easeOutCubic),
    bondGreenBlue().opacity(0.6, 1.5, easeOutCubic),
    bondBlueRed().opacity(0.6, 1.5, easeOutCubic),
  );

  // Update bond positions after quarks move
  bondRedGreen().points([quarkRed().position(), quarkGreen().position()]);
  bondGreenBlue().points([quarkGreen().position(), quarkBlue().position()]);
  bondBlueRed().points([quarkBlue().position(), quarkRed().position()]);

  yield* waitUntil("Proton Shell Forms");
  yield* all(
    protonShell().opacity(0.4, 1, easeOutCubic),
    titleText().text("2 Up + 1 Down = Proton", 1, easeOutCubic),
  );

  yield* waitUntil("Bonds Pulse");
  // Start looping bond animations that run continuously at constant speed
  yield loop(() =>
    all(
      bondRedGreen().lineDashOffset(0).lineDashOffset(-200, 1, linear),
      bondGreenBlue().lineDashOffset(0).lineDashOffset(-200, 1, linear),
      bondBlueRed().lineDashOffset(0).lineDashOffset(-200, 1, linear),
    ),
  );

  yield* waitFor(1);
  yield* waitUntil("End Proton");
});

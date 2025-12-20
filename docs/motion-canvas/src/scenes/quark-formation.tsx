import { makeScene2D } from "@motion-canvas/2d";
import {
  all,
  createRef,
  easeInOutCubic,
  spawn,
  waitFor,
  waitUntil,
} from "@motion-canvas/core";
import { Hadron } from "../components/Hadron";

// Catppuccin Mocha colors (matching the simulation)
const COLORS = {
  background: "#1e1e2e",
  text: "#cdd6f4",
};

export default makeScene2D(function* (view) {
  view.fill(COLORS.background);

  // Create references
  const proton = createRef<Hadron>();
  const neutron = createRef<Hadron>();

  view.add(
    <>
      {/* Proton */}
      <Hadron
        ref={proton}
        hadronType="proton"
        quarkSize={120}
        radius={400}
        startDistance={800}
        opacity={0}
        showText
      />

      {/* Neutron */}
      <Hadron
        ref={neutron}
        hadronType="neutron"
        quarkSize={120}
        radius={400}
        startDistance={800}
        x={400}
        opacity={0}
        showText
      />
    </>,
  );

  // 1.1. Proton: Fade in
  yield* waitUntil("P: Show");
  yield* proton().opacity(1, 0.5);

  // 1.2. Proton: Move quarks and write equation
  yield* waitUntil("P: Quarks");
  const pTask = yield proton().animateFormation(2);
  yield* waitFor(0.5);
  yield* waitUntil("P: Equation");
  yield* proton().typeBaseEquation(1);

  yield* pTask;
  proton().updateBondPositions();

  // 1.3. Proton: Bonds and shell
  yield* waitUntil("P: Bonds");
  yield proton().animateBonds();
  yield* waitUntil("P: Shell");
  yield* all(proton().showShell(1), proton().typeFullEquation(1));
  yield* waitFor(1);

  // 1.4. Proton: Collapse to label
  yield* waitUntil("P: Label");
  yield* proton().collapseToLabel(0.8);
  yield* waitUntil("P: Colors");
  yield proton().rotateQuarkColors();
  yield* waitFor(2);

  // 1.5. Proton: Scale and move
  yield* waitUntil("P: Move");
  yield* all(
    proton().scale(0.6, 1.5, easeInOutCubic),
    proton().position.x(-1500, 1.5, easeInOutCubic),
  );
  yield* waitFor(1);

  // 2.1. Neutron: Fade in
  yield* waitUntil("N: Show");
  yield* neutron().opacity(1, 0.5);

  // 2.2. Neutron: Move quarks and write equation
  yield* waitUntil("N: Quarks");
  const nTask = yield neutron().animateFormation(2);
  yield* waitFor(0.5);
  yield* waitUntil("N: Equation");
  yield* neutron().typeBaseEquation(0.5);

  yield* nTask;
  neutron().updateBondPositions();

  // 2.3. Neutron: Bonds and shell
  yield* waitUntil("N: Bonds");
  yield neutron().animateBonds();
  yield* waitUntil("N: Shell");
  yield* all(neutron().showShell(1), neutron().typeFullEquation(1));
  yield* waitFor(1);

  // 2.4. Neutron: Collapse to label
  yield* waitUntil("N: Label");
  yield* neutron().collapseToLabel(0.8);
  yield* waitUntil("N: Colors");
  yield neutron().rotateQuarkColors();
  yield* waitFor(2);

  // 2.5. Neutron: Scale and move
  yield* waitUntil("N: Move");
  yield* all(
    proton().position.y(-500, 1.5, easeInOutCubic),
    neutron().scale(0.6, 1.5, easeInOutCubic),
    neutron().position.x(-1500, 1.5, easeInOutCubic),
    neutron().position.y(300, 1.5, easeInOutCubic),
  );
  yield* waitFor(1);

  // 3. End
  yield* waitUntil("End");
});

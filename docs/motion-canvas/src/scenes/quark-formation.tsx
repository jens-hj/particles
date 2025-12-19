import { makeScene2D } from "@motion-canvas/2d";
import {
  all,
  createRef,
  easeInOutCubic,
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

  // Animation sequence
  yield* waitUntil("Title Fade In");
  yield* proton().typeBaseEquation(1);

  yield* waitUntil("Show Quark Types");
  yield* proton().typeBaseEquation(1.5);

  yield* waitUntil("Quarks Move Together");
  yield* proton().animateFormation(2);
  proton().updateBondPositions();

  yield* waitUntil("Proton Shell Forms");
  yield* all(proton().showShell(1), proton().typeFullEquation(1));

  yield* waitUntil("Bonds Pulse");
  yield proton().animateBonds();
  yield* waitFor(1);

  yield* waitUntil("Show Proton Labels");
  yield* proton().collapseToLabel(0.8);
  yield* waitFor(2);

  yield* waitUntil("Scale and Move Proton");
  yield* all(
    proton().scale(0.6, 1.5, easeInOutCubic),
    proton().position.x(-1500, 1.5, easeInOutCubic),
  );
  yield* waitFor(1);

  // Neutron formation
  yield* waitUntil("Show Neutron");
  yield* neutron().opacity(1, 0.5);
  yield* neutron().typeBaseEquation(0.5);

  yield* waitUntil("Neutron Quarks Move Together");
  yield* neutron().animateFormation(2);
  neutron().updateBondPositions();

  yield* waitUntil("Neutron Shell Forms");
  yield* all(neutron().showShell(1), neutron().typeFullEquation(1));

  yield* waitUntil("Neutron Bonds Pulse");
  yield neutron().animateBonds();

  yield* waitUntil("Show Neutron Labels");
  yield* neutron().collapseToLabel(0.8);
  yield* waitFor(2);

  yield* waitUntil("Scale and Move Neutron");
  yield* all(
    proton().position.y(-500, 1.5, easeInOutCubic),
    neutron().scale(0.6, 1.5, easeInOutCubic),
    neutron().position.x(-1500, 1.5, easeInOutCubic),
    neutron().position.y(300, 1.5, easeInOutCubic),
  );
  yield* waitFor(1);

  yield* waitUntil("End");
});

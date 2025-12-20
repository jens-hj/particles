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

  // Kick off each hadron's internal sequence.
  // The scene should only do cross-hadron choreography (movement/scaling/relative placement).
  yield* proton().animateSequence({
    prefix: "P",
    formationSeconds: 2,
    equationDelaySeconds: 0.5,
    baseEquationSeconds: 1,
    fullEquationSeconds: 1,
    collapseSeconds: 0.8,
    colorRotationDelaySeconds: 0.5,
    colorRotationCycles: 1,
    postColorsHoldSeconds: 2,
  });

  // Cross-hadron choreography: Proton move
  yield* waitUntil("P: Move");
  yield* all(
    proton().scale(0.6, 1.5, easeInOutCubic),
    proton().position.x(-1500, 1.5, easeInOutCubic),
  );
  yield* waitFor(1);

  yield* neutron().animateSequence({
    prefix: "N",
    formationSeconds: 2,
    equationDelaySeconds: 0.5,
    baseEquationSeconds: 0.5,
    fullEquationSeconds: 1,
    collapseSeconds: 0.8,
    colorRotationDelaySeconds: 0.5,
    colorRotationCycles: 1,
    postColorsHoldSeconds: 2,
  });

  // Cross-hadron choreography: Neutron move (also repositions proton vertically)
  yield* waitUntil("N: Move");
  yield* all(
    proton().position.y(-500, 1.5, easeInOutCubic),
    neutron().scale(0.6, 1.5, easeInOutCubic),
    neutron().position.x(-1500, 1.5, easeInOutCubic),
    neutron().position.y(300, 1.5, easeInOutCubic),
  );
  yield* waitFor(1);

  // End
  yield* waitUntil("End");
});

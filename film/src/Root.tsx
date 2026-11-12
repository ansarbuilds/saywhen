import React from "react";
import { Composition } from "remotion";
import { SaywhenAnnounce } from "./SaywhenAnnounce";
import { SaywhenDemo } from "./SaywhenDemo";

import { Cancel, Cleanup, FiveStates, HeroLoop, Privacy, Safety, SendStyles } from "./AnimatedLoops";
import loopSpecs from "./loop-specs.json";

const loopComponents: Record<string, React.FC> = {
  "hero-loop": HeroLoop,
  "five-states": FiveStates,
  cleanup: Cleanup,
  "send-styles": SendStyles,
  cancel: Cancel,
  safety: Safety,
  privacy: Privacy,
};

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="SaywhenAnnounce"
        component={SaywhenAnnounce}
        durationInFrames={360}
        fps={30}
        width={1920}
        height={1080}
      />
      <Composition
        id="SaywhenDemo"
        component={SaywhenDemo}
        durationInFrames={1800}
        fps={30}
        width={1920}
        height={1080}
      />
      {loopSpecs.map(({ name, width, height, duration }) => (
        <Composition
          key={name}
          id={name}
          component={loopComponents[name]}
          width={width}
          height={height}
          fps={20}
          durationInFrames={duration * 20}
        />
      ))}
    </>
  );
};

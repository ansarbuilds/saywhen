import React from "react";
import { Composition } from "remotion";
import { SaywhenAnnounce } from "./SaywhenAnnounce";
import { SaywhenDemo } from "./SaywhenDemo";

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
    </>
  );
};

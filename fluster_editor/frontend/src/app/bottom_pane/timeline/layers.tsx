import React from "react";
import { FrameData } from "../../types/timeline";
import "./timeline.scss";

export interface LayerProps {
    frames: FrameData[];
    layerVisibility: boolean;
    updateLayerVisibility: (visibile: boolean) => void;
}

export function Layers() {
    return (<div></div>);
}

export function Layer({ frames, layerVisibility }: LayerProps) {
    return (
        <div>
            <div>
                {/*label, visibility*/}
            </div>
            <div>
                {/*render frames in layer*/}
            </div>
        </div>
    );
}

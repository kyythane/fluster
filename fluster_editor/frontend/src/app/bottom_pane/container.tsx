import React from "react";
import { Tab, Tabs } from "@blueprintjs/core";
import { Timeline } from "./timeline/timeline";

export function BottomPaneContainer() {
  return (
    <Tabs
      animate={true}
      id="bottom_pane_tabs"
      renderActiveTabPanelOnly={true}
      selectedTabId="timeline"
    >
      <Tab id="timeline" title="Timeline" panel={<Timeline />} />
    </Tabs>
  );
}

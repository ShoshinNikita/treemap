// Project imports.
import {
  DefaultMarker,
  MapControl,
  MapEventHandler,
  SideBar,
  TreeMarkers,
  TreeSidePane,
  WithHeader,
  WithSidebar,
} from "@/components";

// Local imports.
import { useHomePage } from "./hooks";
import "./styles.css";

export const HomePage = () => {
  const {
    handleViewChange,
    mapState,
    sideBarMode,
    showTree,
  } = useHomePage();

  return (
    <div className="HomePage">
      <WithHeader>
        <WithSidebar>
          <MapControl
            center={mapState.center}
            zoom={mapState.zoom}
          >
            <MapEventHandler
              onViewChange={handleViewChange}
            />

            <TreeMarkers />

            {showTree && (
              <DefaultMarker center={showTree.position} />
            )}
          </MapControl>

          {sideBarMode === "tree" && showTree && (
            <SideBar>
              <TreeSidePane id={showTree.id} />
            </SideBar>
          )}
        </WithSidebar>
      </WithHeader>
    </div>
  );
};

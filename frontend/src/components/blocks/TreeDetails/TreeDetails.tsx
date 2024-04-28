import { ITreeDetails } from "@/types";
import { treeMapService } from "@/services/api";

import "./styles.scss";

interface IProps {
  tree: ITreeDetails;
}

export const TreeDetails = (props: IProps) => {
  const state = props.tree.state ?? "healthy";

  const format = (value: number | null) => {
    if (!value) {
      return "no data";
    }

    return `${value} m`;
  };

  const image_id = props.tree.thumbnail_id;

  return (
    <div className={`TreeDetails state-${state}`}>
      {image_id && (
        <img className="thumbnail" src={treeMapService.getFileURL(image_id)} alt="preview" />
      )}

      <div className="inside">
        <h2>{props.tree.species}</h2>

        <div className="treeId">#{props.tree.id}</div>

        {props.tree.notes && (
          <h3>{props.tree.notes}</h3>
        )}

        <div className="props">
          <div>Height: {format(props.tree.height)}</div>
          <div>Circumference: {format(props.tree.circumference)}</div>
          <div>Canopy: {format(props.tree.diameter)}</div>
          <div>State: {state}</div>
        </div>

        <ul className="links">
          {props.tree.osm_id && (
            <li><a href={`https://www.openstreetmap.org/node/${props.tree.osm_id}`} target="_blank">OSM</a></li>
          )}
          <li><a href={`https://www.wikidata.org/w/index.php?search=${props.tree.species}`} target="_blank">Wikidata</a></li>
        </ul>
      </div>
    </div>
  );
};

import MarkerClusterGroup from "react-leaflet-cluster";

interface IProps {
  children: React.ReactNode | React.ReactNode[];
}

export const MarkerCluster = (props: IProps) => {
  return (
    <MarkerClusterGroup maxClusterRadius={20}>{props.children}</MarkerClusterGroup>
  );
}

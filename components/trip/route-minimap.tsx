"use client";

import "leaflet/dist/leaflet.css";
import { MapContainer, TileLayer, Polyline } from "react-leaflet";
import type { LatLngBoundsExpression } from "leaflet";

// Non-interactive tile map for cards. All gestures off; the wrapping card owns
// pointer events (see pointer-events-none on the caller) so clicks navigate.
export default function RouteMiniMap({ track, className }: { track: [number, number][]; className?: string }) {
  if (track.length < 2) return null;
  const lats = track.map((t) => t[0]);
  const lons = track.map((t) => t[1]);
  const bounds: LatLngBoundsExpression = [
    [Math.min(...lats), Math.min(...lons)],
    [Math.max(...lats), Math.max(...lons)],
  ];
  return (
    <MapContainer
      bounds={bounds}
      boundsOptions={{ padding: [12, 12] }}
      dragging={false}
      touchZoom={false}
      doubleClickZoom={false}
      scrollWheelZoom={false}
      boxZoom={false}
      keyboard={false}
      zoomControl={false}
      attributionControl={false}
      className={className}
    >
      <TileLayer url="https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer/tile/{z}/{y}/{x}" maxNativeZoom={16} />
      <Polyline positions={track} pathOptions={{ color: "#c42e63", weight: 3 }} />
    </MapContainer>
  );
}

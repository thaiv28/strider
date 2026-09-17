"use client";

import { useEffect, useState } from "react";
import { Card } from "@/components/ui";
import { addDays, prettyDate as pretty, clock, describe, isFarFuture, type DailyWx, type Climo } from "@/lib/weather";
import { getClimatology } from "@/app/weather/actions";

type Site = { night: number; lat: number | null; lon: number | null };

export function WeatherPanel({
  lat,
  lon,
  startDate,
  days,
  nights,
  campsites,
  endCoord,
  elevationFt,
}: {
  lat: number | null;
  lon: number | null;
  startDate: string | null;
  days: number;
  nights: number;
  campsites: Site[];
  endCoord: { lat: number | null; lon: number | null } | null;
  elevationFt: number | null;
}) {
  type Row = { key: string; label: string; note?: string; lat: number | null; lon: number | null; date: string };
  let rows: Row[] = [];
  if (startDate && campsites.length) {
    rows = campsites.map((c, i) => ({ key: `d${i}`, label: `Day ${i + 1}`, lat: c.lat, lon: c.lon, date: addDays(startDate, i) }));
    if (endCoord && endCoord.lat != null && endCoord.lon != null)
      rows.push({ key: "fin", label: `Day ${campsites.length + 1}`, note: "finish", lat: endCoord.lat, lon: endCoord.lon, date: addDays(startDate, nights) });
  } else if (startDate && lat != null && lon != null) {
    rows = Array.from({ length: days }, (_, i) => ({ key: `d${i}`, label: days > 1 ? `Day ${i + 1}` : "Forecast", lat, lon, date: addDays(startDate, i) }));
  }

  return (
    <div>
      <div className="eyebrow">Weather</div>
      <Card className="mt-2 p-4">
        {rows.length === 0 ? (
          <div className="text-sm text-muted">
            {startDate ? "Add a route or coordinates to the trip to load weather." : "Set a start date to load weather."}
          </div>
        ) : (
          <div className="divide-y rounded-lg border">
            {rows.map(({ key, ...r }) => (
              <WeatherRow key={key} {...r} elevationFt={elevationFt} />
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}

function WeatherRow({
  label,
  note,
  lat,
  lon,
  date,
  elevationFt,
}: {
  label: string;
  note?: string;
  lat: number | null;
  lon: number | null;
  date: string;
  elevationFt: number | null;
}) {
  const [d, setD] = useState<DailyWx | null>(null);
  const [climo, setClimo] = useState<Climo | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [kind, setKind] = useState<string>("");

  useEffect(() => {
    setD(null);
    setClimo(null);
    if (lat == null || lon == null || !Number.isFinite(lat) || !Number.isFinite(lon)) {
      setMsg("no coordinates");
      return;
    }
    const today = new Date().toISOString().slice(0, 10);
    const past = date < today;
    if (isFarFuture(date)) {
      setMsg(null);
      setKind("typical");
      let live = true;
      getClimatology(lat, lon, date).then((c) => {
        if (!live) return;
        if (c) setClimo(c);
        else setMsg("no climate data");
      });
      return () => { live = false; };
    }
    const vars =
      "weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max,sunrise,sunset" +
      (past ? "" : ",precipitation_probability_max");
    const base = past ? "https://archive-api.open-meteo.com/v1/archive" : "https://api.open-meteo.com/v1/forecast";
    const elev = elevationFt != null ? `&elevation=${Math.round(elevationFt / 3.28084)}` : "";
    setMsg(null);
    setKind(past ? "historical" : "forecast");
    fetch(
      `${base}?latitude=${lat}&longitude=${lon}&start_date=${date}&end_date=${date}&daily=${vars}` +
        `&temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch&timezone=auto${elev}`,
    )
      .then((r) => r.json())
      .then((j) => (j?.daily?.time?.length ? setD(j.daily) : setMsg("no data")))
      .catch(() => setMsg("unavailable"));
  }, [lat, lon, date, elevationFt]);

  const wx = describe(d?.weather_code?.[0]);
  const yr = lat != null && lon != null ? `https://www.yr.no/en/forecast/daily-table/${lat.toFixed(4)},${lon.toFixed(4)}` : null;

  return (
    <div className="flex items-center gap-4 px-4 py-3">
      <div className="w-24 shrink-0">
        <div className="font-display font-semibold">
          {label}
          {note && <span className="ml-1 text-xs font-normal text-muted">· {note}</span>}
        </div>
        <div className="readout text-xs text-muted">{pretty(date)}</div>
        <div className="readout text-[0.65rem] text-muted">
          {lat != null && lon != null ? `${lat.toFixed(3)}, ${lon.toFixed(3)}` : "—"}
        </div>
      </div>

      {msg ? (
        <div className="text-sm text-muted">{msg}</div>
      ) : climo ? (
        <>
          <div className="flex w-40 shrink-0 items-center gap-3">
            <span className="text-3xl leading-none">{climo.snowPct >= 20 ? "❄️" : "📅"}</span>
            <div>
              <div className="text-sm font-medium">Typical</div>
              <div className="readout text-lg font-bold leading-tight">
                {climo.hiF}°<span className="text-sm font-normal text-muted"> / {climo.loF}°</span>
              </div>
            </div>
          </div>
          <div className="hidden flex-1 grid-cols-4 gap-x-6 text-sm text-muted sm:grid">
            <Metric label="Avg precip">{climo.precipIn.toFixed(2)}″</Metric>
            <Metric label="Wet days">{climo.precipPct}%</Metric>
            {climo.snowPct > 0 && <Metric label="Snow days">{climo.snowPct}%</Metric>}
          </div>
          <div className="ml-auto shrink-0 text-right">
            <div className="eyebrow">typical</div>
            <div className="readout text-[0.65rem] text-muted">{climo.years}-yr avg</div>
          </div>
        </>
      ) : !d ? (
        <div className="text-sm text-muted">Loading…</div>
      ) : (
        <>
          <div className="flex w-40 shrink-0 items-center gap-3">
            <span className="text-3xl leading-none">{wx.icon}</span>
            <div>
              <div className="text-sm font-medium">{wx.text}</div>
              <div className="readout text-lg font-bold leading-tight">
                {Math.round(d.temperature_2m_max[0])}°
                <span className="text-sm font-normal text-muted"> / {Math.round(d.temperature_2m_min[0])}°</span>
              </div>
            </div>
          </div>

          <div className="hidden flex-1 grid-cols-4 gap-x-6 text-sm text-muted sm:grid">
            <Metric label="Precip">
              {d.precipitation_sum[0]?.toFixed(2)}″{d.precipitation_probability_max?.[0] != null && ` · ${d.precipitation_probability_max[0]}%`}
            </Metric>
            <Metric label="Wind">{Math.round(d.wind_speed_10m_max[0])} mph</Metric>
            <Metric label="Sunrise">{clock(d.sunrise[0])}</Metric>
            <Metric label="Sunset">{clock(d.sunset[0])}</Metric>
          </div>

          <div className="ml-auto shrink-0 text-right">
            {kind && <div className="eyebrow">{kind}</div>}
            {yr && (
              <a href={yr} target="_blank" rel="noreferrer" className="text-sm text-accent hover:underline">
                forecast ↗
              </a>
            )}
          </div>
        </>
      )}
    </div>
  );
}

function Metric({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <div className="eyebrow">{label}</div>
      <div className="readout">{children}</div>
    </div>
  );
}

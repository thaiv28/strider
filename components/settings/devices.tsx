import { Card, Button } from "@/components/ui";
import { revokeDevice } from "@/app/settings/actions";
type Device = { id: number; deviceName: string; createdAt: Date; lastUsedAt: Date | null };
const date = (value: Date) => value.toLocaleDateString("en-US", { year: "numeric", month: "short", day: "numeric", timeZone: "UTC" });
export function Devices({ devices }: { devices: Device[] }) {
  return <Card className="min-w-0 p-4 sm:p-5">
    <h2 className="text-[17px] font-bold">Devices</h2>
    <div className="mt-4 space-y-2">
      {devices.length === 0 && <p className="text-sm text-muted">No desktop devices are signed in.</p>}
      {devices.map((device) => <div key={device.id} className="flex items-center gap-3 border-b border-divider py-2 last:border-b-0">
        <div className="min-w-0 flex-1">
          <p className="break-words text-sm font-semibold">{device.deviceName}</p>
          <p className="text-xs text-muted">Created {date(device.createdAt)} · Last used {device.lastUsedAt ? date(device.lastUsedAt) : "Never"}</p>
        </div>
        <form action={revokeDevice.bind(null, device.id)}><Button type="submit" variant="danger">Revoke</Button></form>
      </div>)}
    </div>
  </Card>;
}

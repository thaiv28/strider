"use client";

import { Card, SegmentedControl } from "@/components/ui";
import { useTheme } from "@/components/theme-provider";

export function AppearanceSetting() {
  const { preference, setPreference } = useTheme();
  return (
    <Card className="p-4">
      <h2 className="text-[17px] font-bold">Appearance</h2>
      <SegmentedControl
        label="Theme"
        className="mt-3"
        options={[{ value: "light", label: "Light" }, { value: "dark", label: "Dark" }, { value: "system", label: "System" }]}
        value={preference}
        onChange={setPreference}
      />
    </Card>
  );
}

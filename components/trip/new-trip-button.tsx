"use client";

import { useRouter } from "next/navigation";
import { useTransition } from "react";
import { Button } from "@/components/ui";
import { createTrip } from "@/app/trips/actions";

export function NewTripButton() {
  const router = useRouter();
  const [pending, start] = useTransition();
  return (
    <Button
      disabled={pending}
      onClick={() => {
        const name = prompt("Trip name:");
        if (!name) return;
        start(async () => {
          const id = await createTrip(name);
          router.push(`/trips/${id}`);
        });
      }}
    >
      + New trip
    </Button>
  );
}

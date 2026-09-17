import { getCurrentUserId } from "@/lib/gear";
import { getIngredients, getMeals } from "@/lib/pantry";
import { PantryView } from "@/components/food/pantry-view";

export const dynamic = "force-dynamic";
export const metadata = { title: "Food" };

export default async function FoodPage() {
  const userId = await getCurrentUserId();
  const [ingredients, meals] = await Promise.all([getIngredients(userId), getMeals(userId)]);
  return <PantryView ingredients={ingredients} meals={meals} />;
}

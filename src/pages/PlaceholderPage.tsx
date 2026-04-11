import type { LucideIcon } from "lucide-react";

interface PlaceholderPageProps {
  icon: LucideIcon;
  title: string;
  description: string;
}

export default function PlaceholderPage({ icon: Icon, title, description }: PlaceholderPageProps) {
  return (
    <div className="mx-auto mt-24 max-w-md text-center">
      <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-xl bg-accent/10 text-accent">
        <Icon className="size-6" />
      </div>
      <h3 className="mb-2 font-heading text-xl">{title}</h3>
      <p className="text-sm leading-relaxed text-muted-foreground">{description}</p>
    </div>
  );
}

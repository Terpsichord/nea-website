import { PropsWithChildren } from "react";

function AccentBox({ size = "md", children }: PropsWithChildren<{ size: "md" | "lg" }>) {
    // TODO: change these to be appropriate responsive sizes
    const width = size === "md" ? "max-w-xl" : "container";

    return (
        <div className={`mx-auto flex-row bg-light-gray text-black rounded-4xl p-8 ${width}`}>
            {children}
        </div>
    );
}

export default AccentBox;

import { PropsWithChildren } from "react";

function AccentBox({ size = "md", children }: PropsWithChildren<{ size: "md" | "lg" }>) {
    // FIXME: change these to be appropriate responsive sizes
    // FIXME: just in general make sure all the project views are displaying properly
    const width = size === "md" ? "max-w-xl" : "container";

    return (
        <div className={`mx-auto flex-row bg-light-gray text-black rounded-4xl p-8 ${width}`}>
            {children}
        </div>
    );
}

export default AccentBox;

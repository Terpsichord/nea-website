import { PropsWithChildren } from "react";

interface ButtonProps {
    color?: "default" | "red";
    onClick: () => void;
}

function Button({ children, color = "default", onClick }: PropsWithChildren<ButtonProps>) {
    const colorStyles = {
        "default": "bg-transparent",
        "red": "bg-red-400 outline-red-700 text-white",
    };

    return (
        <button className={`block outline rounded-xl px-2 ${colorStyles[color]}`} onClick={onClick}>
            {children}
        </button>
    );
}

export default Button;
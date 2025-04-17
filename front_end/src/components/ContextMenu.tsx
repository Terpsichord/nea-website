import { Dispatch, ReactNode, RefObject, useEffect } from "react";

function ContextMenu({ items, parent, setShow }: { items: ReactNode[], parent: RefObject<HTMLElement | null>, setShow: Dispatch<boolean> }) {
    // hide the context menu if clicked away from
    useEffect(() => {
        const handleClick = (e: MouseEvent) => {
            if (!parent.current || !parent.current.contains(e.target as HTMLElement)) {
                setShow(false);
            }
        };

        window.addEventListener("click", handleClick);

        return () => window.removeEventListener("click", handleClick);
    }, []);

    return (
        <div className="absolute w-52 bg-blue-gray outline outline-gray rounded-xl p-4 -translate-x-11/12 z-10">
            <ul>
                {items.map((item) => (
                <li>{item}</li>

                ))} 
                <li></li>
            </ul>
        </div>
    );

}

export default ContextMenu;
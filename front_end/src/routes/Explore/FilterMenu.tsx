import { useRef, useState } from "react";
import ContextMenu from "../../components/ContextMenu";

function FilterMenu() {
    const [showMenu, setShowMenu] = useState(false);
    const menuParent = useRef<HTMLDivElement | null>(null);

    return (
        <div className="ml-auto bg-blue-gray text-xl p-2" ref={menuParent} onClick={() => setShowMenu(true)}>
            Filter
            {showMenu &&
                <ContextMenu parent={menuParent} setShow={setShowMenu}>
                    <div>
                        Sort by
                    </div>
                    <div>
                        Filter by
                    </div>
                </ContextMenu>
            }
        </div>
    )
}

export default FilterMenu;
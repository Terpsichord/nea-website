import { NavLink } from 'react-router';
import { useAuth } from '../auth';

function Navbar() {
    const activeStyle = ({ isActive }: { isActive: boolean }) => isActive ? "underline decoration-1 font-semibold" : "font-semibold";
    const signInStyle = "bg-white text-black font-medium px-4 py-2.5 rounded-3xl decoration-1 decoration-black";

    const auth = useAuth();

    return (
        <nav className="flex max-w-full bg-linear-to-b from-black to-transparent text-xl px-12 py-8 h-28">
            <div className="space-x-10">
                <NavLink className={activeStyle} to="/">{auth.isAuth ? "Dashboard" : "Home"}</NavLink>
                <NavLink className={activeStyle} to="/explore">Explore</NavLink>
                <NavLink className={activeStyle} to="/about">About</NavLink>
            </div>
            <div className="ml-auto">
                {auth.isAuth ?
                    <NavLink className={activeStyle} to="/profile">Profile</NavLink> :
                    <NavLink className={({ isActive }: { isActive: boolean }) => isActive ? `${signInStyle} underline` : signInStyle} to="/signin">Sign-in</NavLink> 
                }
            </div>
        </nav>
    )
}

export default Navbar;

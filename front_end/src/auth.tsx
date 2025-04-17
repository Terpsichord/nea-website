import { useContext, createContext, useState, useEffect, Context, ReactNode } from "react";
import { useLocation, useNavigate } from "react-router";
import { fetchApi } from "./utils";

interface AuthContextType {
    isAuth: boolean;
    signOut: () => Promise<void>;
}

const AuthContext: Context<AuthContextType> = createContext({} as AuthContextType);

function AuthProvider({ children }: { children: ReactNode }) {
    const [isAuth, setIsAuth] = useState(false);
    const location = useLocation();

    useEffect(() => {
        fetchApi("/auth")
            .then((resp) => resp.json())
            .then((data) => {
                console.log({ isAuth: data.isAuth });
                if (data.isAuth !== isAuth) {
                    setIsAuth(data.isAuth)
                }

            })
    }, [location]);

    const navigate = useNavigate();
    async function signOut() {
        await fetchApi("/signout", { method: "POST" });

        navigate("/");
    }

    return <AuthContext.Provider value={{ isAuth, signOut }}>{ children }</AuthContext.Provider>;

};

export default AuthProvider;

export const useAuth = () => {
    return useContext(AuthContext);
};
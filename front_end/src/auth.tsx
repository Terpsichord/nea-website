import { useContext, createContext, useState, useEffect, Context, Dispatch, PropsWithChildren } from "react";
import { useLocation, useNavigate } from "react-router";
import { fetchApi } from "./utils";

interface AuthContextType {
    isAuth: boolean;
    signOut: () => Promise<void>;
    signedOut: boolean,
    setSignedOut: Dispatch<boolean>,
}

const AuthContext: Context<AuthContextType> = createContext({} as AuthContextType);

function AuthProvider({ children }: PropsWithChildren) {
    const [isAuth, setIsAuth] = useState(false);
    const location = useLocation();

    useEffect(() => {
        fetchApi("/profile/auth")
            .then(resp => resp.json())
            .then(data => {
                if (data.isAuth !== isAuth) {
                    setIsAuth(data.isAuth);
                    console.log({ isAuth });
                    console.log({ new: data.isAuth });
                }

            })
    }, [location]);

    const navigate = useNavigate();

    // needed to ensure redirect to / on sign-out (and not to /signin)
    const [signedOut, setSignedOut] = useState(false);

    async function signOut() {
        await fetchApi("/profile/signout", { method: "POST" });

        navigate("/");
        setSignedOut(true);
        setIsAuth(false);
    }

    return <AuthContext.Provider value={{ isAuth, signOut, signedOut, setSignedOut }}>{ children }</AuthContext.Provider>;

};

export default AuthProvider;

export const useAuth = () => {
    return useContext(AuthContext);
};
import { useEffect } from "react";
import { useAuth } from "../auth";
import Dashboard from "./Dashboard";

function Home() {
  const { isAuth, setSignedOut } = useAuth();

  useEffect(() => setSignedOut(false), []);

  if (isAuth) {
    return <Dashboard />
  }

  return (
    <>
    </>
  )
}

export default Home;

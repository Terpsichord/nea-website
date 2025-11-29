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
    <h1 className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-6xl font-bold">
      Cloud IDE
    </h1>
  )
}

export default Home;

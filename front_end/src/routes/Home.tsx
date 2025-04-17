import { useAuth } from "../auth";
import Dashboard from "./Dashboard";

function Home() {
  const { isAuth } = useAuth();

  if (isAuth) {
    return <Dashboard />
  }

  return (
    <>
    </>
  )
}

export default Home;

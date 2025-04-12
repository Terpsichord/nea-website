import { useMemo, StrictMode } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter, Routes, Route, useLocation } from 'react-router';
import './index.css'
import Home from './Home.tsx'
import SignIn from './SignIn.tsx';
import { CookiesProvider } from 'react-cookie';
import Navbar from './Navbar.tsx';
import Profile from './Profile.tsx';
import AuthProvider from './AuthProvider.tsx';

const root = document.getElementById('root')!;

export function useQuery() {
  const { search } = useLocation();

  return useMemo(() => new URLSearchParams(search), [search]);
}

ReactDOM.createRoot(root).render(
  <StrictMode>
    <CookiesProvider>
      <BrowserRouter>
        <AuthProvider>
          <Navbar />
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/signin" element={<SignIn />} />
            <Route path="/profile" element={<Profile />} />
          </Routes>
        </AuthProvider>
      </BrowserRouter>
    </CookiesProvider>

  </StrictMode>,
)


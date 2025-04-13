import { StrictMode } from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter, Routes, Route } from 'react-router';
import './index.css'
import Home from './Home.tsx'
import SignIn from './SignIn.tsx';
import Navbar from './Navbar.tsx';
import Profile from './Profile.tsx';
import AuthProvider from './AuthProvider.tsx';
import User from './User.tsx';

const root = document.getElementById('root')!;

ReactDOM.createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <Navbar />
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/signin" element={<SignIn />} />
          <Route path="/profile" element={<Profile />} />
          <Route path="/user/:username" element={<User />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  </StrictMode>,
)


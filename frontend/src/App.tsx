import { BrowserRouter, Navigate, Route, Routes, useLocation } from 'react-router-dom';
import type { ReactNode } from 'react';
import TitlePage from './pages/TitlePage';
import HomePage from './pages/HomePage';
import DeckBuilderPage from './pages/DeckBuilderPage';
import BattlePage from './pages/BattlePage';
import OnlineLobbyPage from './pages/OnlineLobbyPage';
import ResultPage from './pages/ResultPage';
import PokemonDetailPage from './pages/PokemonDetailPage';
import TeamPreviewPage from './pages/TeamPreviewPage';
import LoginPage from './pages/LoginPage';
import SignupPage from './pages/SignupPage';
import RankingPage from './pages/RankingPage';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import './index.css';

function ProtectedRoute({ children }: { children: ReactNode }) {
  const { session, loading } = useAuth();
  const location = useLocation();

  if (loading) {
    return (
      <div className="min-h-dvh bg-[var(--surface-1)] flex items-center justify-center">
        <div className="text-[var(--text-muted)] text-lg">読み込み中...</div>
      </div>
    );
  }

  if (!session) {
    return <Navigate to="/login" replace state={{ from: location }} />;
  }

  return <>{children}</>;
}

function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/signup" element={<SignupPage />} />
          <Route path="/ranking" element={<ProtectedRoute><RankingPage /></ProtectedRoute>} />
          <Route path="/" element={<ProtectedRoute><TitlePage /></ProtectedRoute>} />
          <Route path="/home" element={<ProtectedRoute><HomePage /></ProtectedRoute>} />
          <Route path="/deck-builder" element={<ProtectedRoute><DeckBuilderPage /></ProtectedRoute>} />
          <Route path="/team-preview" element={<ProtectedRoute><TeamPreviewPage /></ProtectedRoute>} />
          <Route path="/online-lobby" element={<ProtectedRoute><OnlineLobbyPage /></ProtectedRoute>} />
          <Route path="/battle" element={<ProtectedRoute><BattlePage /></ProtectedRoute>} />
          <Route path="/result" element={<ProtectedRoute><ResultPage /></ProtectedRoute>} />
          <Route path="/pokedex/:speciesId" element={<ProtectedRoute><PokemonDetailPage /></ProtectedRoute>} />
          <Route path="*" element={<ProtectedRoute><Navigate to="/home" replace /></ProtectedRoute>} />
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  );
}

export default App;

import React, { useState, useEffect, useMemo } from 'react';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { Program, AnchorProvider, web3, BN } from '@coral-xyz/anchor';
import idl from './idl.json';
import {
  LayoutDashboard,
  PlusCircle,
  FileText,
  Award,
  CheckCircle2,
  XCircle,
  Copy,
  ExternalLink,
  Coins,
  Shield,
  UploadCloud,
  FileDown,
  Clock,
  RefreshCw,
  Search,
  CheckSquare
} from 'lucide-react';
import './App.css';

const API_BASE = 'http://localhost:8000';
const PROGRAM_ID = new web3.PublicKey(idl.address);

function App() {
  // Connection and Wallet
  const { connection } = useConnection();
  const wallet = useWallet();

  // Navigation
  const [activeTab, setActiveTab] = useState('dashboard');
  
  // States
  const [backendStatus, setBackendStatus] = useState(null);
  const [walletBalance, setWalletBalance] = useState(0);
  const [institution, setInstitution] = useState(null);
  const [processes, setProcesses] = useState([]);
  const [selectedProcessId, setSelectedProcessId] = useState(null);
  const [selectedProcessDetail, setSelectedProcessDetail] = useState(null);
  
  // Loading & UI Notifications
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState('');
  const [successMsg, setSuccessMsg] = useState('');
  const [txSignature, setTxSignature] = useState('');

  // Form States - Initialize Institution
  const [instName, setInstName] = useState('Ministerio de Telecomunicaciones');
  const [instPacLimit, setInstPacLimit] = useState(5000000); // 5M SOL

  // Form States - Create Process
  const [procId, setProcId] = useState(1);
  const [procTitle, setProcTitle] = useState('Adquisición de Servidores Cloud de Alta Performance');
  const [procBudget, setProcBudget] = useState(1200000); // 1.2M SOL
  const [procDeadlineSecs, setProcDeadlineSecs] = useState(3600); // 1 hora
  const [procTdrFile, setProcTdrFile] = useState(null);
  const [procTdrBase64, setProcTdrBase64] = useState('');
  const [procTdrFileName, setProcTdrFileName] = useState('');

  // Form States - Submit Offer
  const [offerFile, setOfferFile] = useState(null);
  const [offerBase64, setOfferBase64] = useState('');
  const [offerFileName, setOfferFileName] = useState('');

  // Form States - Evaluate and Award
  const [awardWinnerProvider, setAwardWinnerProvider] = useState('');
  const [awardWinningBidHash, setAwardWinningBidHash] = useState('');
  const [awardFinalScoreHash, setAwardFinalScoreHash] = useState('');

  // Anchor Program Instance
  const program = useMemo(() => {
    if (!wallet || !wallet.publicKey) return null;
    const provider = new AnchorProvider(connection, wallet, {
      preflightCommitment: 'confirmed',
    });
    return new Program(idl, provider);
  }, [connection, wallet]);

  // Derive PDA Helpers
  const getInstitutionPda = (authorityPubkey) => {
    const [pda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from('institution'), authorityPubkey.toBuffer()],
      PROGRAM_ID
    );
    return pda;
  };

  const getProcessPda = (processId) => {
    const idBuffer = new BN(processId).toArrayLike(Buffer, 'le', 8);
    const [pda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from('process'), idBuffer],
      PROGRAM_ID
    );
    return pda;
  };

  const getOfferPda = (processPda, providerPubkey) => {
    const [pda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from('offer'), processPda.toBuffer(), providerPubkey.toBuffer()],
      PROGRAM_ID
    );
    return pda;
  };

  const getResolutionPda = (processPda) => {
    const [pda] = web3.PublicKey.findProgramAddressSync(
      [Buffer.from('resolution'), processPda.toBuffer()],
      PROGRAM_ID
    );
    return pda;
  };

  // Notification helpers
  const showError = (msg) => {
    setErrorMsg(msg);
    setSuccessMsg('');
    setTxSignature('');
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  const showSuccess = (msg, txSig = '') => {
    setSuccessMsg(msg);
    setErrorMsg('');
    setTxSignature(txSig);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  };

  // Copy helper
  const copyToClipboard = (text) => {
    navigator.clipboard.writeText(text);
    showSuccess('¡Copiado al portapapeles!');
  };

  // Convert File to Base64
  const handleFileChange = (e, setFile, setBase64, setFileName) => {
    const file = e.target.files[0];
    if (!file) return;
    
    setFile(file);
    setFileName(file.name);
    
    const reader = new FileReader();
    reader.readAsDataURL(file);
    reader.onload = () => {
      const base64String = reader.result.split(',')[1];
      setBase64(base64String);
    };
    reader.onerror = () => {
      showError('Error al leer el archivo seleccionado');
    };
  };

  // Fetch Connected Wallet Balance
  const updateWalletBalance = async () => {
    if (!wallet.publicKey) {
      setWalletBalance(0);
      return;
    }
    try {
      const balance = await connection.getBalance(wallet.publicKey);
      setWalletBalance(balance / 1e9);
    } catch (err) {
      console.error('Error al obtener balance de wallet:', err);
    }
  };

  // Fetch Institution Info for the connected authority
  const fetchInstitution = async () => {
    if (!wallet.publicKey || !program) {
      setInstitution(null);
      return;
    }
    try {
      const pda = getInstitutionPda(wallet.publicKey);
      const data = await program.account.institution.fetch(pda);
      setInstitution({
        publicKey: pda.toBase58(),
        name: data.name,
        pacBudgetLimit: data.pacBudgetLimit.toNumber(),
        pacBudgetSpent: data.pacBudgetSpent.toNumber(),
        authority: data.authority.toBase58()
      });
    } catch (err) {
      // Si la cuenta no existe o falla, se setea a null
      setInstitution(null);
    }
  };

  // Fetch Backend Status (determina si hay backend real de IPFS)
  const fetchBackendStatus = async () => {
    try {
      const res = await fetch(`${API_BASE}/api/status`);
      if (res.ok) {
        const data = await res.json();
        setBackendStatus(data);
      } else {
        setBackendStatus(null);
      }
    } catch (err) {
      setBackendStatus(null);
    }
  };

  // Fetch All Procurement Processes from Solana
  const fetchProcesses = async () => {
    if (!program) return;
    try {
      const allAccs = await program.account.procurementProcess.all();
      const mapped = allAccs.map(item => ({
        publicKey: item.publicKey.toBase58(),
        id: item.account.id.toNumber(),
        institution: item.account.institution.toBase58(),
        title: item.account.title,
        referentialBudget: item.account.referentialBudget.toNumber(),
        deadline: item.account.deadline.toNumber(),
        status: item.account.status,
        ipfsTdrHash: item.account.ipfsTdrHash,
      }));
      // Ordenar por ID descendente
      mapped.sort((a, b) => b.id - a.id);
      setProcesses(mapped);
    } catch (err) {
      console.error('Error al listar procesos:', err);
    }
  };

  // Fetch Detailed Process
  const fetchProcessDetail = async (id) => {
    if (!program) return;
    setLoading(true);
    try {
      const processPda = getProcessPda(id);
      const processData = await program.account.procurementProcess.fetch(processPda);
      
      // Obtener todas las ofertas que tengan el process_id respectivo
      const allOffers = await program.account.offerAccount.all([
        {
          memcmp: {
            offset: 40, // offset del process_id en OfferAccount (8 discriminator + 32 provider)
            bytes: new BN(id).toArrayLike(Buffer, 'le', 8).toString('base64'),
            encoding: 'base64'
          }
        }
      ]);
      
      const mappedOffers = allOffers.map(o => ({
        publicKey: o.publicKey.toBase58(),
        provider: o.account.provider.toBase58(),
        processId: o.account.processId.toNumber(),
        ipfsProposalHash: o.account.ipfsProposalHash,
        submissionTimestamp: o.account.submissionTimestamp.toNumber(),
        expertiseVerified: o.account.expertiseVerified,
      }));

      // Si el proceso está adjudicado, buscar la resolución
      let resolution = null;
      if (processData.status === 2) {
        const resolutionPda = getResolutionPda(processPda);
        try {
          const resData = await program.account.awardResolution.fetch(resolutionPda);
          resolution = {
            publicKey: resolutionPda.toBase58(),
            winnerProvider: resData.winnerProvider.toBase58(),
            winningBidHash: resData.winningBidHash,
            finalScoreHash: resData.finalScoreHash,
            timestamp: resData.timestamp.toNumber(),
          };
        } catch (e) {
          console.log('No se pudo leer la cuenta de resolución:', e.message);
        }
      }

      setSelectedProcessDetail({
        publicKey: processPda.toBase58(),
        id: id,
        institution: processData.institution.toBase58(),
        title: processData.title,
        referentialBudget: processData.referentialBudget.toNumber(),
        deadline: processData.deadline.toNumber(),
        status: processData.status,
        ipfsTdrHash: processData.ipfsTdrHash,
        offers: mappedOffers,
        resolution: resolution,
      });

      // Poner al primer oferente como ganador tentativo en el combo
      if (mappedOffers.length > 0) {
        setAwardWinnerProvider(mappedOffers[0].provider);
      }
    } catch (err) {
      showError(`Error al cargar detalle de proceso: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };

  // Solicitar airdrop directamente en localhost
  const handleAirdrop = async () => {
    if (!wallet.publicKey) return;
    setLoading(true);
    try {
      const sig = await connection.requestAirdrop(wallet.publicKey, 2 * 1e9); // 2 SOL
      await connection.confirmTransaction(sig, 'confirmed');
      showSuccess('Airdrop de 2 SOL completado con éxito.', sig);
      updateWalletBalance();
    } catch (err) {
      showError(`Error de airdrop: ${err.message}. Asegúrate de que el validador esté corriendo.`);
    } finally {
      setLoading(false);
    }
  };

  // Efectos de carga de datos iniciales
  useEffect(() => {
    fetchBackendStatus();
    if (wallet.publicKey) {
      updateWalletBalance();
      fetchInstitution();
    }
  }, [wallet.publicKey, connection]);

  useEffect(() => {
    if (program) {
      fetchProcesses();
    }
  }, [program]);

  // Action: Register Institution
  const handleInitInstitution = async (e) => {
    e.preventDefault();
    if (!wallet.publicKey || !program) {
      showError('Por favor conecta tu wallet de desarrollador.');
      return;
    }
    setLoading(true);
    try {
      const pda = getInstitutionPda(wallet.publicKey);
      const tx = await program.methods
        .initializeInstitution(instName, new BN(instPacLimit))
        .accounts({
          institution: pda,
          authority: wallet.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();

      showSuccess(`¡Institución registrada correctamente on-chain!`, tx);
      fetchInstitution();
      updateWalletBalance();
    } catch (err) {
      showError(`Error al inicializar la institución: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };

  // Action: Create Process
  const handleCreateProcess = async (e) => {
    e.preventDefault();
    if (!wallet.publicKey || !program) {
      showError('Por favor conecta tu wallet.');
      return;
    }
    if (!procTdrBase64) {
      showError('Por favor, selecciona y sube un archivo TDR (Términos de Referencia).');
      return;
    }
    setLoading(true);
    try {
      // 1. Subir TDR a IPFS mediante el backend (o simulado si no hay conexión)
      let ipfsHash;
      try {
        const ipfsRes = await fetch(`${API_BASE}/api/processes`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            id: Number(procId),
            title: procTitle,
            referential_budget: Number(procBudget),
            deadline_seconds_from_now: Number(procDeadlineSecs),
            tdr_file_base64: procTdrBase64,
            tdr_file_name: procTdrFileName
          })
        });
        if (ipfsRes.ok) {
          const ipfsData = await ipfsRes.json();
          ipfsHash = ipfsData.ipfsHash;
        } else {
          throw new Error('Error en el servicio IPFS');
        }
      } catch (err) {
        console.log('Usando IPFS simulado (backend desconectado)');
        ipfsHash = `QmMockTDR${Math.random().toString(36).substring(2, 15)}`;
      }

      // 2. Calcular deadline
      const current_time = Math.floor(Date.now() / 1000);
      const deadline = current_time + Number(procDeadlineSecs);

      // 3. Obtener PDAs
      const processPda = getProcessPda(procId);
      const institutionPda = getInstitutionPda(wallet.publicKey);

      // 4. Enviar transacción en Solana firmada por la Wallet
      const tx = await program.methods
        .initializeProcess(
          new BN(procId),
          procTitle,
          new BN(procBudget),
          new BN(deadline),
          ipfsHash
        )
        .accounts({
          process: processPda,
          institution: institutionPda,
          authority: wallet.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();

      showSuccess(`Proceso de compra #${procId} convocado con éxito. IPFS CID: ${ipfsHash}`, tx);
      
      // Reset Form
      setProcId(procId + 1);
      setProcTdrFile(null);
      setProcTdrBase64('');
      setProcTdrFileName('');
      
      fetchProcesses();
      fetchInstitution();
      updateWalletBalance();
    } catch (err) {
      showError(`Error al convocar proceso: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };

  // Action: Submit Offer (Provider)
  const handleSubmitOffer = async (e) => {
    e.preventDefault();
    if (!wallet.publicKey || !program) {
      showError('Por favor conecta tu wallet.');
      return;
    }
    const pid = selectedProcessDetail ? selectedProcessDetail.id : '';
    if (!pid) {
      showError('Por favor selecciona un proceso de compra válido.');
      return;
    }
    if (!offerBase64) {
      showError('Por favor selecciona el archivo de propuesta técnica/económica.');
      return;
    }
    
    setLoading(true);
    try {
      // 1. Subir a IPFS
      let ipfsHash;
      try {
        const ipfsRes = await fetch(`${API_BASE}/api/processes/submit_offer`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            process_id: Number(pid),
            provider_type: "A", // Backend de subida genérico
            proposal_file_base64: offerBase64,
            proposal_file_name: offerFileName
          })
        });
        if (ipfsRes.ok) {
          const ipfsData = await ipfsRes.json();
          ipfsHash = ipfsData.ipfsHash;
        } else {
          throw new Error('Error al subir propuesta a IPFS');
        }
      } catch (err) {
        console.log('Usando IPFS simulado (backend desconectado)');
        ipfsHash = `QmMockProposal${Math.random().toString(36).substring(2, 15)}`;
      }

      // 2. Obtener PDAs
      const processPda = getProcessPda(pid);
      const offerPda = getOfferPda(processPda, wallet.publicKey);

      // 3. Firmar y enviar transacción con la wallet
      const tx = await program.methods
        .submitOffer(new BN(pid), ipfsHash)
        .accounts({
          offer: offerPda,
          process: processPda,
          provider: wallet.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();

      showSuccess(`Oferta enviada con éxito para el Proceso #${pid}. IPFS CID: ${ipfsHash}`, tx);
      
      // Reset Form
      setOfferFile(null);
      setOfferBase64('');
      setOfferFileName('');
      
      fetchProcessDetail(pid);
      updateWalletBalance();
    } catch (err) {
      showError(`Error al presentar la propuesta técnica: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };

  // Action: Verify Offer Expertise (Admin / Institution Authority)
  const handleVerifyOffer = async (providerAddressStr, verified) => {
    if (!selectedProcessDetail || !wallet.publicKey || !program) return;
    setLoading(true);
    try {
      const providerPubkey = new web3.PublicKey(providerAddressStr);
      const processPda = getProcessPda(selectedProcessDetail.id);
      const offerPda = getOfferPda(processPda, providerPubkey);
      const institutionPda = getInstitutionPda(wallet.publicKey);

      const tx = await program.methods
        .verifyOfferExpertise(
          new BN(selectedProcessDetail.id),
          providerPubkey,
          verified
        )
        .accounts({
          offer: offerPda,
          process: processPda,
          institution: institutionPda,
          authority: wallet.publicKey,
        })
        .rpc();

      showSuccess(`Calificación de la oferta registrada con éxito (${verified ? 'Aprobado' : 'Rechazado'}).`, tx);
      fetchProcessDetail(selectedProcessDetail.id);
      updateWalletBalance();
    } catch (err) {
      showError(`Error al calificar oferta: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };

  // Action: Award Process (Admin / Institution Authority)
  const handleAwardProcess = async (e) => {
    e.preventDefault();
    if (!selectedProcessDetail || !wallet.publicKey || !program) return;
    if (!awardWinnerProvider) {
      showError('Por favor selecciona el proveedor ganador.');
      return;
    }
    if (!awardWinningBidHash || !awardFinalScoreHash) {
      showError('Por favor ingresa los hashes CID IPFS del presupuesto ganador y acta de calificación.');
      return;
    }
    
    setLoading(true);
    try {
      const winnerPubkey = new web3.PublicKey(awardWinnerProvider);
      const processPda = getProcessPda(selectedProcessDetail.id);
      const winnerOfferPda = getOfferPda(processPda, winnerPubkey);
      const resolutionPda = getResolutionPda(processPda);
      const institutionPda = getInstitutionPda(wallet.publicKey);

      const tx = await program.methods
        .evaluateAndAward(
          new BN(selectedProcessDetail.id),
          winnerPubkey,
          awardWinningBidHash,
          awardFinalScoreHash
        )
        .accounts({
          resolution: resolutionPda,
          process: processPda,
          winnerOffer: winnerOfferPda,
          institution: institutionPda,
          authority: wallet.publicKey,
          systemProgram: web3.SystemProgram.programId,
        })
        .rpc();

      showSuccess(`¡Proceso #${selectedProcessDetail.id} adjudicado exitosamente!`, tx);
      
      // Reset Form
      setAwardWinningBidHash('');
      setAwardFinalScoreHash('');
      
      fetchProcessDetail(selectedProcessDetail.id);
      fetchProcesses();
      updateWalletBalance();
    } catch (err) {
      showError(`Error al adjudicar el proceso: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };

  // Mapeo de estados del proceso
  const getStatusLabel = (statusCode) => {
    switch (statusCode) {
      case 0: return { label: 'Convocatoria Abierta', class: 'convocatoria' };
      case 1: return { label: 'Evaluación Técnica', class: 'evaluacion' };
      case 2: return { label: 'Adjudicado', class: 'adjudicado' };
      default: return { label: 'Desconocido', class: '' };
    }
  };

  // Formato de fechas
  const formatDeadline = (timestamp) => {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = date - now;
    if (diff < 0) {
      return `Expirado el ${date.toLocaleString()}`;
    }
    const mins = Math.floor(diff / 60000);
    const hours = Math.floor(mins / 60);
    const days = Math.floor(hours / 24);
    
    if (days > 0) return `${days}d ${hours % 24}h restantes`;
    if (hours > 0) return `${hours}h ${mins % 60}m restantes`;
    return `${mins}m restantes`;
  };

  return (
    <div className="app-container">
      {/* HEADER SECTION */}
      <header className="app-header">
        <div className="header-title-area">
          <h1>🛡️ IMtBProcurement</h1>
          <p>Consola de Administración de Contratación Pública Transparente e Inmutable en Solana</p>
        </div>
        
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <div className="ipfs-status-badge">
            <div className={`indicator ${backendStatus ? '' : 'simulated'}`}></div>
            <span>IPFS: {backendStatus ? backendStatus.ipfsMode : 'Simulado (Sin Backend)'}</span>
          </div>
          {/* BOTÓN DE WALLET OFICIAL */}
          <WalletMultiButton className="wallet-adapter-button-custom" />
        </div>
      </header>

      {/* GLOBAL NOTIFICATION SECTION */}
      {errorMsg && (
        <div className="notification error">
          <XCircle size={20} />
          <div>
            <strong>Error:</strong> {errorMsg}
          </div>
        </div>
      )}

      {successMsg && (
        <div className="notification success">
          <CheckCircle2 size={20} />
          <div>
            <strong>Éxito:</strong> {successMsg}
            {txSignature && (
              <div style={{ marginTop: '0.5rem', fontSize: '0.85rem' }}>
                <strong>Firma de Transacción Solana:</strong>{' '}
                <span className="mono" style={{ color: '#ffffff', wordBreak: 'break-all' }}>
                  {txSignature}
                </span>
                <a
                  href={`https://explorer.solana.com/tx/${txSignature}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`}
                  target="_blank"
                  rel="noreferrer"
                  className="ipfs-link"
                  style={{ marginLeft: '0.5rem' }}
                >
                  Ver en Explorer <ExternalLink size={12} />
                </a>
              </div>
            )}
          </div>
        </div>
      )}

      {/* TABS NAVIGATION */}
      <nav className="tabs-navigation">
        <button
          className={`tab-btn ${activeTab === 'dashboard' ? 'active' : ''}`}
          onClick={() => { setActiveTab('dashboard'); if (wallet.publicKey) { updateWalletBalance(); fetchInstitution(); } }}
        >
          <LayoutDashboard size={18} />
          Dashboard
        </button>
        <button
          className={`tab-btn ${activeTab === 'create-process' ? 'active' : ''}`}
          onClick={() => { setActiveTab('create-process'); }}
          disabled={!wallet.publicKey || !institution}
        >
          <PlusCircle size={18} />
          Crear Proceso (Admin)
        </button>
        <button
          className={`tab-btn ${activeTab === 'processes-list' ? 'active' : ''}`}
          onClick={() => { setActiveTab('processes-list'); fetchProcesses(); }}
        >
          <FileText size={18} />
          Procesos y Ofertas
        </button>
      </nav>

      {/* TAB CONTENT: DASHBOARD */}
      {activeTab === 'dashboard' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem' }}>
          
          {/* WALLET STATUS AREA */}
          <div className="glass-card">
            <h2 className="glass-card-title"><Coins size={20} /> Tu Billetera Conectada (Localnet)</h2>
            {wallet.publicKey ? (
              <div className="wallets-grid" style={{ gridTemplateColumns: '1fr' }}>
                <div className={`wallet-item ${institution ? 'authority' : 'provider-a'}`}>
                  <div className="wallet-header">
                    <span className="wallet-role" style={{ color: institution ? 'var(--accent-cyan)' : 'var(--accent-green)' }}>
                      {institution ? 'Autoridad Pública Registrada' : 'Proveedor / Licitante Público'}
                    </span>
                    <Shield size={16} color={institution ? 'var(--accent-cyan)' : 'var(--accent-green)'} />
                  </div>
                  <div className="wallet-balance">
                    {walletBalance.toFixed(4)} <span>SOL</span>
                  </div>
                  
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '1rem' }}>
                    <div className="wallet-address">
                      {wallet.publicKey.toBase58().slice(0, 12)}...{wallet.publicKey.toBase58().slice(-12)}
                      <button className="copy-btn" onClick={() => copyToClipboard(wallet.publicKey.toBase58())}>
                        <Copy size={12} />
                      </button>
                    </div>
                    
                    <button className="btn btn-secondary" onClick={handleAirdrop} disabled={loading} style={{ padding: '0.4rem 0.8rem', fontSize: '0.85rem' }}>
                      {loading ? 'Fondeando...' : 'Solicitar +2 SOL Airdrop'}
                    </button>
                  </div>
                </div>
              </div>
            ) : (
              <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-secondary)' }}>
                <Shield size={48} style={{ margin: '0 auto 1rem', opacity: 0.3 }} />
                <h3>Billetera no conectada</h3>
                <p style={{ marginTop: '0.5rem', marginBottom: '1.5rem' }}>Conecta tu wallet de Phantom en el botón superior derecho para interactuar con la red.</p>
                <div style={{ display: 'inline-block' }}>
                  <WalletMultiButton />
                </div>
              </div>
            )}
          </div>
 
          {/* INSTITUTION INITIALIZATION / CONFIG */}
          {wallet.publicKey && (
            <div className="glass-card">
              <h2 className="glass-card-title"><Shield size={20} /> Institución Pública Asociada a tu Wallet</h2>
              {institution ? (
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1.5rem', padding: '0.5rem' }}>
                  <div className="meta-row">
                    <span className="meta-label">Nombre de la Entidad</span>
                    <span className="meta-value" style={{ fontSize: '1.25rem', fontWeight: '700' }}>
                      {institution.name}
                    </span>
                  </div>
                  <div className="meta-row">
                    <span className="meta-label">Límite Presupuestario PAC</span>
                    <span className="meta-value" style={{ fontSize: '1.25rem', fontWeight: '700', color: 'var(--accent-cyan)' }}>
                      {institution.pacBudgetLimit.toLocaleString()} SOL
                    </span>
                  </div>
                  <div className="meta-row">
                    <span className="meta-label">Presupuesto Devengado</span>
                    <span className="meta-value" style={{ fontSize: '1.25rem', fontWeight: '700', color: 'var(--accent-orange)' }}>
                      {institution.pacBudgetSpent.toLocaleString()} SOL
                    </span>
                  </div>
                  <div className="meta-row">
                    <span className="meta-label">Dirección PDA On-Chain</span>
                    <span className="meta-value mono" style={{ fontSize: '0.8rem' }}>
                      {institution.publicKey}
                    </span>
                  </div>
                </div>
              ) : (
                <div>
                  <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem' }}>
                    Tu billetera no está registrada como una Autoridad de Licitación. Si representas a una Entidad Gubernamental, puedes registrarla en Solana completando el siguiente formulario.
                  </p>
                  <form onSubmit={handleInitInstitution} className="form-grid">
                    <div className="form-group">
                      <label>Nombre de la Entidad Pública</label>
                      <input
                        type="text"
                        value={instName}
                        onChange={(e) => setInstName(e.target.value)}
                        required
                      />
                    </div>
                    <div className="form-group">
                      <label>Límite Techo Anual PAC (SOL)</label>
                      <input
                        type="number"
                        value={instPacLimit}
                        onChange={(e) => setInstPacLimit(e.target.value)}
                        required
                      />
                    </div>
                    <div className="form-group" style={{ display: 'flex', alignItems: 'flex-end' }}>
                      <button type="submit" className="btn btn-primary" style={{ width: '100%' }} disabled={loading}>
                        {loading ? <div className="loader"></div> : 'Registrar Entidad Gubernamental'}
                      </button>
                    </div>
                  </form>
                </div>
              )}
            </div>
          )}

          {/* DOCUMENTACIÓN / AYUDA */}
          <div className="glass-card">
            <h2 className="glass-card-title"><FileText size={20} /> Flujo de Control Transparente (MVP)</h2>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem', color: 'var(--text-secondary)', fontSize: '0.85rem', lineHeight: '1.4' }}>
              <div style={{ background: 'rgba(255,255,255,0.01)', padding: '1rem', borderRadius: '8px' }}>
                <h4 style={{ color: 'var(--text-primary)', marginBottom: '0.5rem' }}>1. Convocatoria</h4>
                La Autoridad registra el proceso especificando el presupuesto y subiendo el TDR a IPFS. Esto descuenta/reserva presupuesto del techo del PAC.
              </div>
              <div style={{ background: 'rgba(255,255,255,0.01)', padding: '1rem', borderRadius: '8px' }}>
                <h4 style={{ color: 'var(--text-primary)', marginBottom: '0.5rem' }}>2. Oferta de Proveedores</h4>
                Los proveedores suben sus propuestas a IPFS antes de la fecha límite y registran su firma digital on-chain.
              </div>
              <div style={{ background: 'rgba(255,255,255,0.01)', padding: '1rem', borderRadius: '8px' }}>
                <h4 style={{ color: 'var(--text-primary)', marginBottom: '0.5rem' }}>3. Calificación Técnica</h4>
                El comité técnico audita la propuesta on-chain y califica el cumplimiento de los requerimientos técnicos del proveedor.
              </div>
              <div style={{ background: 'rgba(255,255,255,0.01)', padding: '1rem', borderRadius: '8px' }}>
                <h4 style={{ color: 'var(--text-primary)', marginBottom: '0.5rem' }}>4. Adjudicación Inmutable</h4>
                Superada la evaluación, se adjudica el proceso al ganador subiendo el acta final y confirmando el compromiso de fondos definitivo.
              </div>
            </div>
          </div>

        </div>
      )}

      {/* TAB CONTENT: CREATE PROCESS */}
      {activeTab === 'create-process' && (
        <div className="glass-card">
          <h2 className="glass-card-title"><PlusCircle size={20} /> Convocar Nuevo Proceso de Contratación</h2>
          <form onSubmit={handleCreateProcess} style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
            <div className="form-grid">
              <div className="form-group">
                <label>ID del Proceso (Numérico Único)</label>
                <input
                  type="number"
                  value={procId}
                  onChange={(e) => setProcId(Number(e.target.value))}
                  required
                />
              </div>
              <div className="form-group" style={{ gridColumn: 'span 2' }}>
                <label>Objeto de la Contratación / Título</label>
                <input
                  type="text"
                  value={procTitle}
                  onChange={(e) => setProcTitle(e.target.value)}
                  required
                />
              </div>
              <div className="form-group">
                <label>Presupuesto Referencial (SOL)</label>
                <input
                  type="number"
                  value={procBudget}
                  onChange={(e) => setProcBudget(Number(e.target.value))}
                  required
                />
              </div>
              <div className="form-group">
                <label>Plazo Límite (Segundos desde ahora)</label>
                <select
                  value={procDeadlineSecs}
                  onChange={(e) => setProcDeadlineSecs(Number(e.target.value))}
                >
                  <option value={60}>1 Minuto (Pruebas Rápidas)</option>
                  <option value={300}>5 Minutos</option>
                  <option value={3600}>1 Hora</option>
                  <option value={86400}>24 Horas</option>
                  <option value={604800}>7 Días</option>
                </select>
              </div>
            </div>

            <div className="form-group">
              <label>Términos de Referencia (TDR) / Pliego Base (PDF, DOC, etc.)</label>
              <div className="file-upload-zone">
                <UploadCloud size={32} />
                <span>
                  {procTdrFile ? `Archivo seleccionado: ${procTdrFileName}` : 'Arrastra o selecciona el pliego de condiciones de contratación'}
                </span>
                <input
                  type="file"
                  onChange={(e) => handleFileChange(e, setProcTdrFile, setProcTdrBase64, setProcTdrFileName)}
                  style={{ display: 'none' }}
                  id="tdr-file-picker"
                />
                <button
                  type="button"
                  className="btn btn-secondary"
                  style={{ padding: '0.4rem 1rem', fontSize: '0.85rem' }}
                  onClick={() => document.getElementById('tdr-file-picker').click()}
                >
                  Seleccionar Archivo
                </button>
                {procTdrBase64 && (
                  <span className="file-upload-meta">
                    ✓ Preparado para codificación y subida IPFS ({Math.round(procTdrBase64.length * 0.75 / 1024)} KB)
                  </span>
                )}
              </div>
            </div>

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem', marginTop: '1rem' }}>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => setActiveTab('dashboard')}
              >
                Cancelar
              </button>
              <button
                type="submit"
                className="btn btn-primary"
                disabled={loading}
              >
                {loading ? <div className="loader"></div> : 'Publicar Convocatoria en Solana'}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* TAB CONTENT: PROCESSES AND OFFERS */}
      {activeTab === 'processes-list' && (
        <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1.8fr', gap: '1.5rem', alignItems: 'start' }}>
          
          {/* LEFT COLUMN: LIST OF PROCESSES */}
          <div className="glass-card">
            <h2 className="glass-card-title"><Search size={20} /> Convocatorias On-Chain</h2>
            {processes.length > 0 ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                {processes.map((p) => (
                  <div
                    key={p.id}
                    className={`offer-card ${selectedProcessId === p.id ? 'active' : ''}`}
                    style={{
                      cursor: 'pointer',
                      border: selectedProcessId === p.id ? '1px solid var(--accent-cyan)' : '1px solid var(--border-color)',
                      background: selectedProcessId === p.id ? 'rgba(0, 229, 255, 0.03)' : 'rgba(30, 41, 59, 0.2)'
                    }}
                    onClick={() => {
                      setSelectedProcessId(p.id);
                      fetchProcessDetail(p.id);
                    }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                      <span style={{ fontWeight: '700', fontSize: '0.95rem' }}>Proceso #{p.id}</span>
                      <span className={`status-pill ${getStatusLabel(p.status).class}`}>
                        {getStatusLabel(p.status).label}
                      </span>
                    </div>
                    <p style={{ fontSize: '0.9rem', color: 'var(--text-primary)', fontWeight: '600' }}>{p.title}</p>
                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                      <span>Presupuesto: {p.referentialBudget.toLocaleString()} SOL</span>
                      <span>{formatDeadline(p.deadline)}</span>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p style={{ color: 'var(--text-secondary)', textAlign: 'center', padding: '2rem' }}>
                No se encontraron convocatorias de compras en el programa Solana.
              </p>
            )}
          </div>

          {/* RIGHT COLUMN: DETAILED VIEW OR GENERAL BID SUBMISSION */}
          <div>
            {selectedProcessDetail ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                
                {/* PROCESS METADATA */}
                <div className="glass-card">
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <h2 className="glass-card-title" style={{ border: 'none', padding: '0' }}>
                      Detalle de Adjudicación: #{selectedProcessDetail.id}
                    </h2>
                    <button
                      className="btn btn-secondary"
                      style={{ padding: '0.35rem 0.75rem', fontSize: '0.8rem' }}
                      onClick={() => fetchProcessDetail(selectedProcessDetail.id)}
                    >
                      <RefreshCw size={12} /> Actualizar
                    </button>
                  </div>
                  
                  <div className="process-detail-grid">
                    <div className="meta-list">
                      <div className="meta-row">
                        <span className="meta-label">Título del Proceso</span>
                        <span className="meta-value">{selectedProcessDetail.title}</span>
                      </div>
                      <div className="meta-row">
                        <span className="meta-label">Presupuesto</span>
                        <span className="meta-value">{selectedProcessDetail.referentialBudget.toLocaleString()} SOL</span>
                      </div>
                      <div className="meta-row">
                        <span className="meta-label">Estado actual</span>
                        <div>
                          <span className={`status-pill ${getStatusLabel(selectedProcessDetail.status).class}`}>
                            {getStatusLabel(selectedProcessDetail.status).label}
                          </span>
                        </div>
                      </div>
                    </div>
                    
                    <div className="meta-list">
                      <div className="meta-row">
                        <span className="meta-label">Plazo límite</span>
                        <span className="meta-value">{formatDeadline(selectedProcessDetail.deadline)}</span>
                      </div>
                      <div className="meta-row">
                        <span className="meta-label">TDR Pliego de Condiciones</span>
                        <div>
                          <a
                            href={
                              backendStatus
                                ? `https://gateway.pinata.cloud/ipfs/${selectedProcessDetail.ipfsTdrHash}`
                                : '#'
                            }
                            target="_blank"
                            rel="noreferrer"
                            className="ipfs-link"
                          >
                            <FileDown size={14} /> Descargar TDR ({selectedProcessDetail.ipfsTdrHash.slice(0, 12)}...)
                          </a>
                        </div>
                      </div>
                      <div className="meta-row">
                        <span className="meta-label">PDA Cuenta Solana</span>
                        <span className="meta-value mono">{selectedProcessDetail.publicKey}</span>
                      </div>
                    </div>
                  </div>

                  {/* AWARD RESOLUTION DISPLAY (IF AWARDED) */}
                  {selectedProcessDetail.status === 2 && selectedProcessDetail.resolution && (
                    <div style={{ marginTop: '1rem', padding: '1rem', border: '1px solid var(--accent-green)', background: 'rgba(16, 185, 129, 0.05)', borderRadius: '12px' }}>
                      <h4 style={{ color: 'var(--accent-green)', fontFamily: 'var(--font-heading)', fontWeight: '700', display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.75rem' }}>
                        <Award size={18} /> PROCESO ADJUDICADO EXITOSAMENTE
                      </h4>
                      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', fontSize: '0.85rem' }}>
                        <div className="meta-row">
                          <span className="meta-label">Adjudicatario Ganador</span>
                          <span className="meta-value mono" style={{ color: '#ffffff' }}>
                            {selectedProcessDetail.resolution.winnerProvider}
                          </span>
                        </div>
                        <div className="meta-row">
                          <span className="meta-label">Fecha de Adjudicación</span>
                          <span className="meta-value">
                            {new Date(selectedProcessDetail.resolution.timestamp * 1000).toLocaleString()}
                          </span>
                        </div>
                        <div className="meta-row" style={{ gridColumn: 'span 2' }}>
                          <span className="meta-label">CID Presupuesto Definitivo (IPFS)</span>
                          <span className="meta-value mono">
                            <a
                              href={backendStatus ? `https://gateway.pinata.cloud/ipfs/${selectedProcessDetail.resolution.winningBidHash}` : '#'}
                              target="_blank"
                              rel="noreferrer"
                              className="ipfs-link"
                            >
                              {selectedProcessDetail.resolution.winningBidHash} <ExternalLink size={12} />
                            </a>
                          </span>
                        </div>
                        <div className="meta-row" style={{ gridColumn: 'span 2' }}>
                          <span className="meta-label">CID Acta de Puntuación Final (IPFS)</span>
                          <span className="meta-value mono">
                            <a
                              href={backendStatus ? `https://gateway.pinata.cloud/ipfs/${selectedProcessDetail.resolution.finalScoreHash}` : '#'}
                              target="_blank"
                              rel="noreferrer"
                              className="ipfs-link"
                            >
                              {selectedProcessDetail.resolution.finalScoreHash} <ExternalLink size={12} />
                            </a>
                          </span>
                        </div>
                      </div>
                    </div>
                  )}
                </div>

                {/* OFFERS RECEIVED FOR THIS PROCESS */}
                <div className="glass-card">
                  <h2 className="glass-card-title"><FileText size={20} /> Ofertas Presentadas ({selectedProcessDetail.offers ? selectedProcessDetail.offers.length : 0})</h2>
                  {selectedProcessDetail.offers && selectedProcessDetail.offers.length > 0 ? (
                    <div className="offers-list">
                      {selectedProcessDetail.offers.map((offer) => (
                        <div key={offer.provider} className="offer-card">
                          <div className="offer-header">
                            <span style={{ fontWeight: '700', fontSize: '0.85rem' }}>
                              Proveedor:{' '}
                              {wallet.publicKey && offer.provider === wallet.publicKey.toBase58()
                                ? 'Tu Wallet (Tú)'
                                : `${offer.provider.slice(0, 10)}...${offer.provider.slice(-10)}`}
                            </span>
                            <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                              Enviado: {new Date(offer.submissionTimestamp * 1000).toLocaleTimeString()}
                            </span>
                          </div>
                          <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 0.8fr', gap: '1rem', alignItems: 'center' }}>
                            <div className="meta-row">
                              <span className="meta-label">Propuesta Técnica / Económica (IPFS)</span>
                              <a
                                href={backendStatus ? `https://gateway.pinata.cloud/ipfs/${offer.ipfsProposalHash}` : '#'}
                                target="_blank"
                                rel="noreferrer"
                                className="ipfs-link"
                                style={{ fontSize: '0.85rem' }}
                              >
                                Descargar Oferta ({offer.ipfsProposalHash.slice(0, 10)}...)
                              </a>
                            </div>
                            
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', alignItems: 'flex-end' }}>
                              <span className="meta-label">Evaluación Técnica</span>
                              {offer.expertiseVerified ? (
                                <span className="status-pill adjudicado" style={{ fontSize: '0.75rem' }}>
                                  <CheckSquare size={12} /> Aprobada / Cumple
                                </span>
                              ) : (
                                <span className="status-pill convocatoria" style={{ fontSize: '0.75rem', background: 'rgba(239, 68, 68, 0.1)', color: 'var(--accent-red)', borderColor: 'rgba(239, 68, 68, 0.2)' }}>
                                  No Calificado / Rechazado
                                </span>
                              )}
                            </div>
                          </div>

                          {/* ACTION: QUALIFY/VERIFY OFFER EXPERTISE (ONLY AUTHORITY ADMIN) */}
                          {selectedProcessDetail.status === 1 && institution && wallet.publicKey && selectedProcessDetail.institution === institution.publicKey && (
                            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem', marginTop: '0.5rem', borderTop: '1px solid rgba(255,255,255,0.02)', paddingTop: '0.5rem' }}>
                              <button
                                className="btn btn-danger"
                                style={{ padding: '0.35rem 0.75rem', fontSize: '0.75rem', borderRadius: '4px' }}
                                onClick={() => handleVerifyOffer(offer.provider, false)}
                                disabled={loading}
                              >
                                Marcar No Cumple
                              </button>
                              <button
                                className="btn btn-primary"
                                style={{ padding: '0.35rem 0.75rem', fontSize: '0.75rem', borderRadius: '4px', background: 'var(--accent-green)', color: 'white', boxShadow: 'none' }}
                                onClick={() => handleVerifyOffer(offer.provider, true)}
                                disabled={loading}
                              >
                                Aprobar Requisitos
                              </button>
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', textAlign: 'center', padding: '1rem' }}>
                      No se han presentado ofertas para esta convocatoria.
                    </p>
                  )}
                </div>

                {/* SUBMIT NEW OFFER (IF ACTIVE STATUS == 0 AND NOT MET DEADLINE) */}
                {selectedProcessDetail.status === 0 && wallet.publicKey && (
                  <div className="glass-card">
                    <h2 className="glass-card-title"><UploadCloud size={20} /> Presentar Propuesta (Portal de Proveedores)</h2>
                    <form onSubmit={handleSubmitOffer} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                      <div className="form-grid">
                        <div className="form-group" style={{ gridColumn: 'span 3' }}>
                          <label>Documento de Oferta Técnica/Económica (PDF, ZIP)</label>
                          <div className="file-upload-zone" style={{ padding: '1rem' }}>
                            <UploadCloud size={24} />
                            <span style={{ fontSize: '0.85rem' }}>
                              {offerFile ? `Propuesta: ${offerFileName}` : 'Selecciona el documento de tu propuesta'}
                            </span>
                            <input
                              type="file"
                              onChange={(e) => handleFileChange(e, setOfferFile, setOfferBase64, setOfferFileName)}
                              style={{ display: 'none' }}
                              id="offer-file-picker"
                            />
                            <button
                              type="button"
                              className="btn btn-secondary"
                              style={{ padding: '0.3rem 0.75rem', fontSize: '0.75rem' }}
                              onClick={() => document.getElementById('offer-file-picker').click()}
                            >
                              Cargar Archivo
                            </button>
                          </div>
                        </div>
                      </div>
                      
                      <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.5rem' }}>
                        <button type="submit" className="btn btn-primary" disabled={loading}>
                          {loading ? <div className="loader"></div> : 'Subir y Firmar Oferta'}
                        </button>
                      </div>
                    </form>
                  </div>
                )}

                {/* AWARD PROCESS FORM (IF STATUS == 1 (EVALUATION)) */}
                {selectedProcessDetail.status === 1 && institution && wallet.publicKey && selectedProcessDetail.institution === institution.publicKey && (
                  <div className="glass-card" style={{ border: '1px solid var(--accent-orange)' }}>
                    <h2 className="glass-card-title" style={{ color: 'var(--accent-orange)' }}><Award size={20} /> Resolución de Adjudicación (Comité de Compras)</h2>
                    <form onSubmit={handleAwardProcess} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                      <div className="form-grid">
                        <div className="form-group">
                          <label>Proveedor Seleccionado Ganador</label>
                          <select
                            value={awardWinnerProvider}
                            onChange={(e) => setAwardWinnerProvider(e.target.value)}
                            required
                          >
                            <option value="">-- Seleccionar Ganador --</option>
                            {selectedProcessDetail.offers && selectedProcessDetail.offers.map(o => (
                              <option key={o.provider} value={o.provider}>
                                {o.provider.slice(0, 12)}...{o.provider.slice(-12)}
                              </option>
                            ))}
                          </select>
                        </div>
                        <div className="form-group">
                          <label>Hash CID del Presupuesto Definitivo (IPFS)</label>
                          <input
                            type="text"
                            placeholder="Ej. QmWinningProposalDetail..."
                            value={awardWinningBidHash}
                            onChange={(e) => setAwardWinningBidHash(e.target.value)}
                            required
                          />
                        </div>
                        <div className="form-group">
                          <label>Hash CID Acta de Calificación (IPFS)</label>
                          <input
                            type="text"
                            placeholder="Ej. QmFinalEvaluationAct..."
                            value={awardFinalScoreHash}
                            onChange={(e) => setAwardFinalScoreHash(e.target.value)}
                            required
                          />
                        </div>
                      </div>

                      <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.5rem' }}>
                        <button type="submit" className="btn btn-primary" style={{ background: 'var(--accent-orange)', color: '#000000', boxShadow: 'none' }} disabled={loading}>
                          {loading ? <div className="loader"></div> : 'Confirmar Adjudicación y Comprometer Fondos'}
                        </button>
                      </div>
                    </form>
                  </div>
                )}

              </div>
            ) : (
              <div className="glass-card" style={{ textAlign: 'center', padding: '3rem', color: 'var(--text-secondary)' }}>
                <FileText size={48} style={{ margin: '0 auto 1rem', opacity: 0.3 }} />
                <h3>Selecciona una convocatoria de la lista</h3>
                <p style={{ fontSize: '0.9rem', marginTop: '0.5rem' }}>
                  Haz clic en cualquier proceso de compra en la columna de la izquierda para ver su detalle técnico, ofertas recibidas, calificar a los proveedores y adjudicar el contrato.
                </p>
              </div>
            )}
          </div>

        </div>
      )}

      {/* FOOTER METADATA */}
      <footer style={{ marginTop: 'auto', paddingTop: '2rem', borderTop: '1px solid var(--border-color)', textAlign: 'center', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
        <p>Tesis de Maestría en Blockchain & Web3 - Desarrollado en Rust y Rocket (Backend de IPFS) + Anchor y Solana (Smart Program) + Vite & React (Frontend)</p>
        <p style={{ marginTop: '0.25rem', fontFamily: 'var(--font-mono)' }}>
          Program ID: {PROGRAM_ID.toBase58()}
        </p>
      </footer>
    </div>
  );
}

export default App;

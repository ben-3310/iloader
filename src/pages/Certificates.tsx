import "./Certificates.css";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

type Certificate = {
  name: string;
  certificateId: string;
  serialNumber: string;
  machineName: string;
  machineId: string;
};

export const Certificates = () => {
  const [certificates, setCertificates] = useState<Certificate[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const loadingRef = useRef<boolean>(false);

  const loadCertificates = useCallback(async () => {
    if (loadingRef.current) return;
    const promise = async () => {
      loadingRef.current = true;
      setLoading(true);
      try {
        // Используем кэшированную версию для быстрой загрузки
        let certs = await invoke<Certificate[]>("get_certificates_cached");
        // Валидация данных сертификатов
        certs = certs.map(cert => ({
          name: cert.name || "Unknown",
          certificateId: cert.certificateId || "",
          serialNumber: cert.serialNumber || "",
          machineName: cert.machineName || "",
          machineId: cert.machineId || "",
        })).filter(cert => cert.certificateId !== "" && cert.serialNumber !== "");

        setCertificates(certs);
      } catch (error: any) {
        console.error("Error loading certificates:", error);
        const errorMsg = String(error);

        // Специальная обработка ошибок парсинга machineId
        if (errorMsg.includes("machineId") || errorMsg.includes("Parse") || errorMsg.includes("machineld")) {
          const detailedError =
            "Ошибка парсинга данных от Apple API (machineId).\n\n" +
            "Это известная проблема, которая может возникать из-за изменений в формате API Apple.\n\n" +
            "Возможные решения:\n" +
            "1. Выйдите и войдите снова в приложении\n" +
            "2. Отзовите все существующие сертификаты и создайте новые\n" +
            "3. Проверьте наличие обновлений iloader\n" +
            "4. Сообщите об этой проблеме разработчикам\n\n" +
            "Техническая информация: " + errorMsg;
          throw new Error(detailedError);
        }
        throw error;
      } finally {
        setLoading(false);
        loadingRef.current = false;
      }
    };
    toast.promise(promise, {
      loading: "Загрузка сертификатов...",
      success: "Сертификаты успешно загружены!",
      error: (e) => {
        const errorStr = String(e);
        // Если ошибка содержит многострочное сообщение, показываем его полностью
        if (errorStr.includes("\n")) {
          return errorStr;
        }
        return "Не удалось загрузить сертификаты: " + errorStr;
      },
    });
  }, [setCertificates]);

  const revokeCertificate = useCallback(
    async (serialNumber: string) => {
      const promise = invoke<void>("revoke_certificate", {
        serialNumber,
      });
      promise.then(loadCertificates);
      toast.promise(promise, {
        loading: "Revoking certificate...",
        success: "Certificate revoked successfully!",
        error: (e) => "Failed to revoke certificate: " + e,
      });
    },
    [setCertificates, loadCertificates]
  );

  useEffect(() => {
    loadCertificates();
  }, []);

  return (
    <>
      <h2>Manage Certificates</h2>
      {certificates.length === 0 ? (
        <div>
          {loading ? "Loading certificates..." : "No certificates found."}
        </div>
      ) : (
        <div className="card">
          <div className="certificate-table-container">
            <table className="certificate-table">
              <thead>
                <tr className="certificate-item">
                  <th className="cert-item-part">Name</th>
                  <th className="cert-item-part">Serial Number</th>
                  <th className="cert-item-part">Machine Name</th>
                  <th className="cert-item-part">Machine ID</th>
                  <th>Revoke</th>
                </tr>
              </thead>
              <tbody>
                {certificates.map((cert, i) => (
                  <tr
                    key={cert.certificateId}
                    className={
                      "certificate-item" +
                      (i === certificates.length - 1 ? " cert-item-last" : "")
                    }
                  >
                    <td className="cert-item-part">{cert.name}</td>
                    <td className="cert-item-part">{cert.serialNumber}</td>
                    <td className="cert-item-part">{cert.machineName}</td>
                    <td className="cert-item-part">{cert.machineId && cert.machineId.trim() !== "" ? cert.machineId : "—"}</td>
                    <td
                      className="cert-item-revoke"
                      onClick={() => revokeCertificate(cert.serialNumber)}
                    >
                      Revoke
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
      <button
        style={{ marginTop: "1em" }}
        onClick={loadCertificates}
        disabled={loading}
      >
        Refresh
      </button>
    </>
  );
};

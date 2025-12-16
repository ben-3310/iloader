import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { toast } from "sonner";
import "./Certificates.css";

type CleanupResult = {
  certificatesRevoked: number;
  appIdsDeleted: number;
  errors: string[];
};

export const Cleanup = () => {
  const [loading, setLoading] = useState<boolean>(false);
  const [result, setResult] = useState<CleanupResult | null>(null);

  const performCleanup = useCallback(async () => {
    if (loading) return;

    const confirmed = window.confirm(
      "⚠️ ВНИМАНИЕ: Это действие удалит ВСЕ сертификаты и App ID!\n\n" +
        "Это действие нельзя отменить. Вы уверены, что хотите продолжить?"
    );

    if (!confirmed) return;

    const promise = async () => {
      setLoading(true);
      setResult(null);
      const cleanupResult = await invoke<CleanupResult>("cleanup_all");
      setResult(cleanupResult);
      setLoading(false);
      return cleanupResult;
    };

    toast.promise(promise, {
      loading: "Очистка всех данных...",
      success: (result: CleanupResult) => {
        const successMsg =
          `Очистка завершена!\n` +
          `Отозвано сертификатов: ${result.certificatesRevoked}\n` +
          `Удалено App ID: ${result.appIdsDeleted}`;

        if (result.errors.length > 0) {
          return successMsg + `\nОшибок: ${result.errors.length}`;
        }
        return successMsg;
      },
      error: (e) => `Очистка не удалась: ${e}`,
    });
  }, [loading]);

  return (
    <>
      <h2>Очистка всех данных</h2>
      <div className="card" style={{ marginBottom: "1em" }}>
        <p style={{ marginBottom: "1em", lineHeight: "1.6" }}>
          <strong>Эта функция выполнит:</strong>
        </p>
        <ul style={{ marginLeft: "1.5em", lineHeight: "1.8" }}>
          <li>Отзовет все сертификаты разработчика</li>
          <li>Удалит все App ID</li>
        </ul>
        <p style={{ marginTop: "1em", color: "#ff6b6b", fontWeight: "bold" }}>
          ⚠️ ВНИМАНИЕ: Это действие нельзя отменить!
        </p>
      </div>

      <button
        onClick={performCleanup}
        disabled={loading}
        style={{
          padding: "0.75em 1.5em",
          fontSize: "1em",
          backgroundColor: loading ? "#666" : "#ff6b6b",
          color: "white",
          border: "none",
          borderRadius: "8px",
          cursor: loading ? "not-allowed" : "pointer",
          fontWeight: "bold",
        }}
      >
        {loading ? "Очистка..." : "Начать очистку всех данных"}
      </button>

      {result && (
        <div className="card" style={{ marginTop: "1.5em" }}>
          <h3 style={{ marginTop: 0 }}>Результаты очистки</h3>
          <div style={{ lineHeight: "1.8" }}>
            <p>
              <strong>Отозвано сертификатов:</strong>{" "}
              {result.certificatesRevoked}
            </p>
            <p>
              <strong>Удалено App ID:</strong> {result.appIdsDeleted}
            </p>
            {result.errors.length > 0 && (
              <div style={{ marginTop: "1em" }}>
                <strong style={{ color: "#ff6b6b" }}>
                  Ошибки ({result.errors.length}):
                </strong>
                <ul style={{ marginLeft: "1.5em", marginTop: "0.5em" }}>
                  {result.errors.map((error, i) => (
                    <li key={i} style={{ color: "#ff6b6b", fontSize: "0.9em" }}>
                      {error}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
};

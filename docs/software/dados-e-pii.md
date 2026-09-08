# Dados e PII

## Onde

| Coisa | Sítio | Git | OneDrive |
|---|---|---|---|
| Fichas, JSON, fotos de clientes | `Documents\Vanguarda\Dados\Clientes` | Nunca | Nunca |
| Chave AES | `Dados\Clientes\.vanguarda-key` | Nunca (`.gitignore` já ignora `*.key`) | Nunca |
| Logins (pessoas + software) | `Interno\ops.sqlite`, tabela `pessoa` | Nunca | Nunca |
| `utilizadores.json` | leftover; a app migra uma vez para `pessoa` e deixa de o usar | Nunca | Nunca |
| Orçamentos / OS / stock | o mesmo `ops.sqlite` | Nunca | Nunca |
| Auditoria | `Interno\audit.jsonl` | Nunca | Nunca |
| Último backup | `Interno\ultimo-backup.txt` | Nunca | Nunca |
| Academia | `Documents\Vanguarda\Educacao` | Só o índice `docs/educacao.md` | Não precisa |
| Legal / NIF empresa / NDAs | OneDrive da empresa | Nunca | Sim, conta empresa |
| Código e planos | `Documents\Vanguarda\Software` / GitHub ElegantVW | Sim (sem PII) | Não |

`alha.toml` ao lado do exe: `home` + `clientes`. Variáveis: `VANGUARDA_HOME`, `FICHA_CLIENTES_DIR`.

## Vault

Campos de contacto (NIF, telemóvel, email, morada) nas fichas vão prefixados `enc:v1:` com AES-256-GCM. A chave é um ficheiro de 32 bytes na raiz dos clientes. Sem a chave, a app mostra lixo ou o ciphertext.

Cópia de segurança = zip da pasta **e** da chave, no mesmo sítio seguro (disco local ou pen da casa). Perder a chave = PII ilegível.

## AppData

A instalação antiga usava `%LOCALAPPDATA%\Vanguarda\Clientes`. A casa é Documents. Confirmar uma ficha (Inês) a partir de `Dados\` e só depois apagar o leftover do AppData. Ver `CASA.txt`.

## Diagnóstico

Logs AT/ST (quando existirem) em `Interno\DiagLogs\`. Sem NIF. Relatórios do cliente ficam na pasta da pessoa / do carro.

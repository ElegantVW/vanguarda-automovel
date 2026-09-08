# Manual — Design e administrador do software

## Conta do software ≠ mesa Design

O sysadmin do **programa** é um login à parte (`software: true`). Não é o Gil Salvador, não é o Rodrigo, não é a mesa Design.

Design é Front: estilo dos PDF, guias internas, identidade. Quem faz Design no dia-a-dia entra com o login da **pessoa** e escolhe a mesa Design no cabeçalho. Relatório (Autocom → PDF da casa) também se escreve nesta mesa — não é preciso mudar para Oficina.

## First-run (casa vazia)

Pasta de dados vazia → «Primeira conta». Marca **Conta do software**. Nome por omissão `Vanguarda`. Depois, em Sistema, crias as pessoas (Gil Salvador · Escritório; Rodrigo Sousa · Oficina).

Logins Design antigos, sem a flag admin, ainda são promovidos a admin na primeira leitura **se ninguém tiver admin** (migração). Não depende do nome.

## Sistema

Só admin / conta do software.

- Pessoas novas: só em **Pessoas** (pedido → aprovar/recusar). Sistema não cria humanos.
- **Conta do software** (sem pasta), repor passe, desactivar login.
- Mudar mesa, activo, overlay admin (evitar na pessoa — usa a conta do software).
- Repor passe, apagar **login** (a pasta da pessoa fica).
- **Eliminar outras pessoas:** fica Gil Salvador + conta do software. Pastas de staff a mais apagam-se. Clientes não se tocam.
- Logins vivem na tabela `pessoa` de `Interno\ops.sqlite` (já era o sítio dos orçamentos). Sem Postgres.
- Não se apaga nem se desactiva o último administrador.

A ficha humana edita-se em **Pessoas**. Sistema não duplica o formulário.

## Documentos

Estilo Formal vs Estilizado, fundo, tinta, ícones. Pré-visualizar e «Guardar estilo». Os outros departamentos só vêem o resultado nos PDF. O fundo da **app** é charcoal; os JPEG de tecido ficam para o papel.

Fundos e ícones: `Documents\Vanguarda\Design\`.

## Backup

«Backup zip» (admin). Destino por omissão `Documents\Vanguarda\Backups\`. Não mandar o zip para OneDrive se tiver fichas. A app avisa no login se o último backup tiver mais de 7 dias (ou nunca). Cada backup, login, orçamento aceite, ficha gravada e união de pastas fica em `Interno\audit.jsonl`. Palavras-passe novas são Argon2; logins SHA-256 antigos passam a Argon2 no primeiro login correcto.

## Compilar e instalar

Ver [compilar.md](compilar.md). Um só `Documents\Vanguarda\App\Vanguarda.exe`. Manter `alha.toml`. A janela abre maximizada; o primeiro arranque regista o início de sessão.

## Git

Sítio unificado: `Documents\Vanguarda\Software` → [ElegantVW/vanguarda-automovel](https://github.com/ElegantVW/vanguarda-automovel). Não meter Dados no git.

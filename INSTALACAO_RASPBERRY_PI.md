# Guia de Instalação do Rosav no Raspberry Pi

Este guia fornece instruções passo a passo para instalar e executar o Rosav em um Raspberry Pi rodando Debian/Ubuntu.

## Pré-requisitos

- Raspberry Pi com sistema operacional baseado em Debian (Debian 12+ ou Ubuntu 22.04+)
- Acesso root ou sudo
- Conexão com a internet
- Pelo menos 4GB de RAM recomendado (o build pode ser demorado)

## Passo 1: Instalação do Rust

O Rosav é desenvolvido em Rust. Vamos instalar a toolchain do Rust usando o `rustup`:

```bash
# Baixar e instalar o rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Recarregar as variáveis de ambiente
source "$HOME/.cargo/env"

# Verificar a instalação
rustc --version
cargo --version
```

**Nota:** Se você já tem o Rust instalado, certifique-se de que está usando uma versão compatível (1.59 ou superior).

## Passo 2: Instalação das Dependências do Sistema

### 2.1 Dependências Básicas

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    libwebkit2gtk-4.0-dev \
    libjavascriptcoregtk-4.0-dev \
    libappindicator3-dev \
    librsvg2-dev \
    patchelf \
    libudev-dev \
    pkg-config \
    libssl-dev
```

### 2.2 WebKitGTK 4.0 (Importante!)

O Rosav requer WebKitGTK 4.0 (que usa libsoup2), mas sistemas mais recentes podem ter apenas WebKitGTK 4.1 (libsoup3). Se você estiver em Debian 13 (Trixie) ou superior, será necessário instalar o WebKitGTK 4.0 do repositório do Debian 12 (Bookworm):

```bash
# Adicionar repositório do Debian Bookworm
echo "deb http://deb.debian.org/debian bookworm main" | sudo tee /etc/apt/sources.list.d/bookworm.list

# Atualizar lista de pacotes
sudo apt-get update

# Instalar WebKitGTK 4.0 do Bookworm
sudo apt-get install -t bookworm \
    libwebkit2gtk-4.0-37 \
    libwebkit2gtk-4.0-dev \
    libjavascriptcoregtk-4.0-18 \
    libjavascriptcoregtk-4.0-dev
```

**Verificação:**
```bash
# Verificar se o WebKitGTK 4.0 está disponível
pkg-config --modversion webkit2gtk-4.0

# Deve retornar algo como: 2.48.5
```

## Passo 3: Instalação das Ferramentas de Desenvolvimento

### 3.1 Instalar o Tauri CLI

```bash
cargo install tauri-cli

# Verificar instalação
cargo tauri --version
```

**Nota:** A versão recomendada do `tauri-cli` para este projeto é 1.6.3. Se você instalar uma versão mais recente e encontrar erros de configuração, pode instalar especificamente a versão 1.6.3:

```bash
cargo install tauri-cli --version 1.6.3
```

### 3.2 Instalar o Trunk (Bundler para WebAssembly)

```bash
cargo install trunk

# Verificar instalação
trunk --version
```

**Nota:** A versão recomendada do `trunk` é 0.21.4 ou superior.

### 3.3 Adicionar Target WebAssembly

O frontend do Rosav é compilado para WebAssembly:

```bash
rustup target add wasm32-unknown-unknown
```

## Passo 4: Clonar e Configurar o Projeto

### 4.1 Clonar o Repositório

```bash
cd ~
git clone <URL_DO_REPOSITORIO> rosav
cd rosav
```

### 4.2 Configurar o Script de Build do Frontend

Crie um script para evitar problemas com variáveis de ambiente durante o build:

```bash
cat > src-tauri/build_frontend.sh << 'EOF'
#!/bin/bash
# Script wrapper para build do frontend sem problemas de variáveis de ambiente
cd "$(dirname "$0")/../frontend"
unset NO_COLOR
unset FORCE_COLOR
trunk build --release
EOF

chmod +x src-tauri/build_frontend.sh
```

### 4.3 Atualizar Configuração do Tauri

Edite o arquivo `src-tauri/tauri.conf.json` e atualize o `beforeBuildCommand`:

```json
{
  "build": {
    "beforeBuildCommand": "bash /home/litel/rosav/src-tauri/build_frontend.sh",
    ...
  }
}
```

**Nota:** Substitua `/home/litel/rosav` pelo caminho completo do seu projeto.

## Passo 5: Build do Projeto

### 5.1 Build de Desenvolvimento (Opcional - para testar)

```bash
cd ~/rosav/src-tauri

# Limpar builds anteriores (opcional)
cargo clean

# Executar em modo desenvolvimento
unset NO_COLOR FORCE_COLOR
cargo tauri dev
```

Este comando irá:
- Compilar o frontend com `trunk serve`
- Compilar o backend em Rust
- Abrir a aplicação em modo desenvolvimento

**Tempo estimado:** 20-30 minutos na primeira compilação

### 5.2 Build de Produção

```bash
cd ~/rosav/src-tauri

# Limpar builds anteriores
cargo clean

# Build de produção (gera pacote .deb)
unset NO_COLOR FORCE_COLOR
cargo tauri build --bundles deb
```

**Tempo estimado:** 40-60 minutos na primeira compilação

O pacote `.deb` será gerado em:
```
target/release/bundle/deb/rosav_0.1.0_arm64.deb
```

## Passo 6: Instalação do Pacote

### 6.1 Instalar o Pacote .deb

```bash
sudo dpkg -i ~/rosav/src-tauri/target/release/bundle/deb/rosav_0.1.0_arm64.deb
```

Se houver dependências faltando, instale-as:

```bash
sudo apt-get install -f
```

### 6.2 Verificar Instalação

```bash
# Verificar se o executável foi instalado
which rosav

# Deve retornar: /usr/bin/rosav

# Verificar se o pacote está instalado
dpkg -l | grep rosav
```

## Passo 7: Criar Atalho na Área de Trabalho

O pacote já cria um arquivo `.desktop` em `/usr/share/applications/`. Para criar um atalho na área de trabalho:

```bash
# Copiar para a área de trabalho
cp /usr/share/applications/rosav.desktop ~/Desktop/
chmod +x ~/Desktop/rosav.desktop
```

## Passo 8: Executar o Rosav

Agora você pode executar o Rosav de três formas:

### 8.1 Via Atalho da Área de Trabalho
- Dê duplo clique no ícone `rosav.desktop` na área de trabalho

### 8.2 Via Terminal
```bash
rosav
```

### 8.3 Via Menu de Aplicativos
- Procure por "Rosav" no menu de aplicativos do sistema

## Solução de Problemas

### Problema: Erro "libsoup3 symbols detected"

**Sintoma:**
```
libsoup-ERROR **: libsoup3 symbols detected. Using libsoup2 and libsoup3 in the same process is not supported.
```

**Solução:**
Certifique-se de que o WebKitGTK 4.0 está instalado e que o `pkg-config` encontra a versão correta:

```bash
# Verificar versões disponíveis
pkg-config --list-all | grep webkit

# Deve mostrar webkit2gtk-4.0 e webkit2gtk-4.1

# Forçar uso do WebKitGTK 4.0 durante o build
PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig cargo tauri build
```

### Problema: Erro "trunk: not found"

**Solução:**
```bash
cargo install trunk
source "$HOME/.cargo/env"
```

### Problema: Erro "invalid value '1' for '--no-color'"

**Sintoma:**
```
error: invalid value '1' for '--no-color'
[possible values: true, false]
```

**Solução:**
Use o script `build_frontend.sh` criado no Passo 4.2, que remove as variáveis de ambiente problemáticas.

### Problema: Build muito lento

**Soluções:**
- Use `cargo build --release` em vez de `cargo build` para builds de produção
- Aumente a memória swap se necessário
- Feche outros aplicativos durante o build
- Considere usar um Raspberry Pi com mais RAM (8GB recomendado para builds)

### Problema: Janela não abre após a instalação

**Verificações:**
```bash
# Verificar se o DISPLAY está configurado
echo $DISPLAY

# Se não estiver, configure:
export DISPLAY=:0

# Verificar se há processos do Rosav rodando
ps aux | grep rosav

# Verificar logs de erro
rosav 2>&1 | tee /tmp/rosav.log
```

## Requisitos do Sistema

- **RAM:** Mínimo 2GB, recomendado 4GB+
- **Espaço em disco:** Pelo menos 5GB livres (para dependências e build)
- **CPU:** Qualquer modelo do Raspberry Pi (Pi 4 ou superior recomendado)
- **Sistema Operacional:** Debian 12+ ou Ubuntu 22.04+

## Atualização do Rosav

Para atualizar o Rosav após fazer alterações no código:

```bash
cd ~/rosav
git pull  # Se usando git

# Rebuild
cd src-tauri
cargo clean
unset NO_COLOR FORCE_COLOR
cargo tauri build --bundles deb

# Reinstalar
sudo dpkg -i target/release/bundle/deb/rosav_0.1.0_arm64.deb
```

## Desinstalação

Para remover o Rosav do sistema:

```bash
sudo dpkg -r rosav
```

Para remover completamente (incluindo arquivos de configuração):

```bash
sudo dpkg -P rosav
```

## Informações Adicionais

- **Versão do Rust recomendada:** 1.59 ou superior
- **Versão do Tauri CLI:** 1.6.3
- **Versão do Trunk:** 0.21.4 ou superior
- **WebKitGTK:** 4.0 (libsoup2)

## Suporte

Se encontrar problemas durante a instalação, verifique:
1. Todos os pré-requisitos foram instalados corretamente
2. As versões das ferramentas estão compatíveis
3. Há espaço suficiente em disco
4. A conexão com a internet está funcionando (para downloads)

Para mais informações, consulte o arquivo `README.md` do projeto.


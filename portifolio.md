

Criar portifolio com este projeto:
Encrutador de links

Frontend em ingles demonstrando que foi criado tres backends para o mesmo projeto que deve ser totalmente funcional sem erros de sincronizção entre os backends que devem consumir o mesmo banco. deve conter os links para cada backend

backends dentro de containers podman distroless
encurtador de links
- em rust com axum
- em golang com gin e gorm
- em fastapi
com clean arquitect adpatado ao projeto
com clean code
postgres
redis
com swagger
com multi-tenant
com login e 2fa totp
com nginx
com forte segurança 
com proteção contra race condition
com proteção contra N+1



# #8 Desafio de Programação: Encurtador de Links

## **Descrição**

Desenvolva um sistema completo para encurtar links, utilizando **Docker**, **Traefik** como reverse proxy, e um sistema que **identifique URLs encurtadas automaticamente** com base no domínio e estrutura da URL. O desafio inclui funcionalidades de gerenciamento de links, redirecionamento dinâmico e registro de métricas.

---

## **Objetivo**

1. Criar um sistema que permita encurtar links e gerenciá-los.
2. Permitir que links encurtados sejam redirecionados com base apenas no **short_code** presente na URL.
3. Utilizar **Traefik** como reverse proxy para roteamento dinâmico.
4. Configurar todo o ambiente com **Docker Compose** para facilitar o deploy.

---

## **O que será Aprendido**

1. **Redirecionamento Inteligente:**
    - Processar URLs encurtadas diretamente pelo domínio e path.
2. **APIs Avançadas:**
    - Criar endpoints RESTful para gerenciamento de links.
3. **Docker e Containers:**
    - Orquestrar serviços e ambiente de produção com Docker Compose.
4. **Traefik como Reverse Proxy:**
    - Configurar roteamento dinâmico sem rotas adicionais.
5. **Métricas Avançadas:**
    - Registro de acessos (IP, agente de usuário e data/hora).

---

## **Arquitetura**

### **Diagrama de Arquitetura**

![Fluxo do Encurtador de Links](https://racoelho.com.br/assets/challenges/encurtador-de-links/fluxo-encurtador-links.png)

---

## **Especificações Técnicas**

### **Backend**

- **Banco de Dados:** PostgreSQL, MySQL, SQL.
- **Reverse Proxy:** Traefik.
- **Ambiente:** Docker e Docker Compose.
- **Métricas:** Contador de cliques, rastreamento de IPs e agente de usuário.

---

## **Endpoints e Payloads**

### 1. **Criar Link Curto**

- **POST** `/api/shorten`
- **Body:**
    
    `{   "url": "https://exemplo.com/minha-url" }`
    
- **Response:**
    
    `{   "short_url": "https://short.local/abc123",   "original_url": "https://exemplo.com/minha-url" }`
    

---

### 2. **Redirecionamento**

- O sistema reconhece o formato `https://short.local/{short_code}` automaticamente.
- Ao acessar `https://short.local/abc123`, a requisição é roteada para a API, que:
    1. Busca o `short_code` no banco.
    2. Redireciona o cliente para a URL original com código **`302 Found`**.

---

### 3. **Listar Links Criados**

- **GET** `/api/links`
- **Response:**
    
    `[   {     "short_url": "https://short.local/abc123",     "original_url": "https://exemplo.com",     "clicks": 42   },   {     "short_url": "https://short.local/xyz789",     "original_url": "https://outra.com",     "clicks": 15   } ]`
    

---

### 4. **Excluir Link Curto**

- **DELETE** `/api/links/{short_code}`
- **Response:**
    
    `{   "message": "Link deleted successfully" }`
    

---

## **Resultados Esperados**

1. **Encurtador Funcional:**
    - Gerar links curtos e redirecionar automaticamente sem necessidade de rotas explícitas.
2. **Integração com Traefik:**
    - Roteamento dinâmico baseado em `short_code`.
3. **Métricas Avançadas:**
    - Registro de IPs, agentes de usuário e datas de acesso.
4. **Ambiente Contenerizado:**
    - Docker Compose configurado para fácil deploy.

---

Boa sorte com o desafio! 🚀

from sqlalchemy import Column, String, Boolean, BigInteger, Text, DateTime, ForeignKey, Index, Float
from sqlalchemy.dialects.postgresql import UUID, INET
from sqlalchemy.orm import DeclarativeBase, relationship
from sqlalchemy.sql import func
import uuid


class Base(DeclarativeBase):
    __abstract__ = True
    id = Column(UUID(as_uuid=True), primary_key=True, default=uuid.uuid4)
    created_at = Column(DateTime(timezone=True), server_default=func.now())


class Tenant(Base):
    __tablename__ = "tenants"

    name = Column(String(255), nullable=False)
    slug = Column(String(100), nullable=False, unique=True)

    users = relationship("User", back_populates="tenant", cascade="all, delete-orphan")
    links = relationship("Link", back_populates="tenant", cascade="all, delete-orphan")


class User(Base):
    __tablename__ = "users"

    tenant_id = Column(UUID(as_uuid=True), ForeignKey("tenants.id", ondelete="CASCADE"), nullable=False)
    email = Column(String(255), nullable=False)
    password_hash = Column(String(255), nullable=False)
    totp_secret = Column(String(32), nullable=True)
    totp_enabled = Column(Boolean, default=False)

    tenant = relationship("Tenant", back_populates="users")
    links = relationship("Link", back_populates="user")

    __table_args__ = (
        Index("idx_users_tenant_email", "tenant_id", "email", unique=True),
    )


class Link(Base):
    __tablename__ = "links"

    tenant_id = Column(UUID(as_uuid=True), ForeignKey("tenants.id", ondelete="CASCADE"), nullable=False)
    user_id = Column(UUID(as_uuid=True), ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    short_code = Column(String(20), nullable=False, unique=True)
    original_url = Column(Text, nullable=False)
    clicks = Column(BigInteger, default=0)
    expires_at = Column(DateTime(timezone=True), nullable=True)

    tenant = relationship("Tenant", back_populates="links")
    user = relationship("User", back_populates="links")
    click_events = relationship("ClickEvent", back_populates="link", cascade="all, delete-orphan")

    __table_args__ = (
        Index("idx_links_short_code", "short_code"),
        Index("idx_links_tenant_id", "tenant_id"),
    )


class ClickEvent(Base):
    __tablename__ = "click_events"

    link_id = Column(UUID(as_uuid=True), ForeignKey("links.id", ondelete="CASCADE"), nullable=False)
    ip = Column(INET, nullable=True)
    user_agent = Column(Text, nullable=True)
    referer = Column(Text, nullable=True)
    country = Column(String(2), nullable=True)
    city = Column(String(255), nullable=True)
    latitude = Column(Float, nullable=True)
    longitude = Column(Float, nullable=True)
    isp = Column(String(255), nullable=True)
    clicked_at = Column(DateTime(timezone=True), server_default=func.now())

    link = relationship("Link", back_populates="click_events")

    __table_args__ = (
        Index("idx_click_events_link_id", "link_id"),
        Index("idx_click_events_country", "country"),
    )


class Pixel(Base):
    __tablename__ = "pixels"

    tenant_id = Column(UUID(as_uuid=True), ForeignKey("tenants.id", ondelete="CASCADE"), nullable=False)
    user_id = Column(UUID(as_uuid=True), ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    code = Column(String(20), nullable=False, unique=True)
    name = Column(String(255), nullable=True)
    clicks = Column(BigInteger, default=0)

    tenant = relationship("Tenant")
    user = relationship("User")
    pixel_click_events = relationship("PixelClickEvent", back_populates="pixel", cascade="all, delete-orphan")

    __table_args__ = (
        Index("idx_pixels_code", "code"),
        Index("idx_pixels_tenant", "tenant_id"),
    )


class PixelClickEvent(Base):
    __tablename__ = "pixel_click_events"

    pixel_id = Column(UUID(as_uuid=True), ForeignKey("pixels.id", ondelete="CASCADE"), nullable=False)
    ip = Column(INET, nullable=True)
    user_agent = Column(Text, nullable=True)
    referer = Column(Text, nullable=True)
    country = Column(String(2), nullable=True)
    city = Column(String(255), nullable=True)
    latitude = Column(Float, nullable=True)
    longitude = Column(Float, nullable=True)
    isp = Column(String(255), nullable=True)
    clicked_at = Column(DateTime(timezone=True), server_default=func.now())

    pixel = relationship("Pixel", back_populates="pixel_click_events")

    __table_args__ = (
        Index("idx_pixel_click_events_pixel_id", "pixel_id"),
    )
